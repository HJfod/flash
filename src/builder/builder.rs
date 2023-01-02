use clang::{Entity, EntityKind};
use indicatif::ProgressBar;
use std::{collections::HashMap, fs, sync::Arc};
use strfmt::strfmt;
use tokio::task::JoinHandle;

use crate::{
    config::Config,
    html::html::{GenHtml, Html, HtmlElement, HtmlList, HtmlText},
    url::UrlPath,
};

use super::{files::Root, index::Index, namespace::Namespace};

pub trait EntityMethods<'e> {
    fn rel_url(&self) -> UrlPath {
        UrlPath::new_with_path(self.get_fully_qualified_name())
    }
    fn docs_url(&self, config: Arc<Config>) -> UrlPath {
        self.rel_url().to_absolute(config)
    }
    fn get_fully_qualified_name(&self) -> Vec<String>;
    fn get_ancestorage(&self) -> Vec<Entity<'e>>;
}

impl<'e> EntityMethods<'e> for Entity<'e> {
    fn get_fully_qualified_name(&self) -> Vec<String> {
        self.get_ancestorage()
            .iter()
            .map(|a| a.get_name().unwrap_or("_anon".into()))
            .collect()
    }

    fn get_ancestorage(&self) -> Vec<Entity<'e>> {
        let mut ancestors = Vec::new();
        if let Some(parent) = self.get_semantic_parent() {
            match parent.get_kind() {
                EntityKind::TranslationUnit
                | EntityKind::UnexposedDecl
                | EntityKind::UnexposedAttr
                | EntityKind::UnexposedExpr
                | EntityKind::UnexposedStmt => {}
                _ => ancestors.extend(parent.get_ancestorage()),
            }
        }
        ancestors.push(self.clone());
        ancestors
    }
}

pub enum NavItem {
    Root(Option<String>, Vec<NavItem>),
    Dir(String, Vec<NavItem>, Option<(String, bool)>),
    Link(String, UrlPath, Option<(String, bool)>),
}

impl NavItem {
    pub fn new_link(name: &str, url: UrlPath, icon: Option<(&str, bool)>) -> NavItem {
        NavItem::Link(name.into(), url, icon.map(|s| (s.0.into(), s.1)))
    }

    pub fn new_dir(name: &str, items: Vec<NavItem>, icon: Option<(&str, bool)>) -> NavItem {
        NavItem::Dir(name.into(), items, icon.map(|s| (s.0.into(), s.1)))
    }

    pub fn new_root(name: Option<&str>, items: Vec<NavItem>) -> NavItem {
        NavItem::Root(name.map(|s| s.into()), items)
    }

    pub fn to_html(&self, config: Arc<Config>) -> Html {
        match self {
            NavItem::Link(name, url, icon) => HtmlElement::new("a")
                .with_attr(
                    "onclick",
                    &format!("return navigate('{}')", url.to_absolute(config.clone())),
                )
                .with_attr("href", url.to_absolute(config))
                .with_child_opt(icon.as_ref().map(|i| {
                    HtmlElement::new("i")
                        .with_attr("data-feather", &i.0)
                        .with_class("icon")
                        .with_class_opt(i.1.then_some("variant"))
                }))
                .with_child(HtmlText::new(name))
                .into(),

            NavItem::Dir(name, items, icon) => HtmlElement::new("details")
                .with_child(
                    HtmlElement::new("summary")
                        .with_child(
                            HtmlElement::new("i").with_attr("data-feather", "chevron-right"),
                        )
                        .with_child_opt(icon.as_ref().map(|i| {
                            HtmlElement::new("i")
                                .with_attr("data-feather", &i.0)
                                .with_class("icon")
                                .with_class_opt(i.1.then_some("variant"))
                        }))
                        .with_child(HtmlText::new(name)),
                )
                .with_child(
                    HtmlElement::new("div")
                        .with_children(items.iter().map(|i| i.to_html(config.clone())).collect()),
                )
                .into(),

            NavItem::Root(name, items) => {
                if let Some(name) = name {
                    HtmlElement::new("details")
                        .with_attr("open", "")
                        .with_attr("class", "root")
                        .with_child(
                            HtmlElement::new("summary")
                                .with_child(
                                    HtmlElement::new("i")
                                        .with_attr("data-feather", "chevron-right"),
                                )
                                .with_child(HtmlText::new(name)),
                        )
                        .with_child(HtmlElement::new("div").with_children(
                            items.iter().map(|i| i.to_html(config.clone())).collect(),
                        ))
                        .into()
                } else {
                    HtmlList::new(items.iter().map(|i| i.to_html(config.clone())).collect()).into()
                }
            }
        }
    }
}

pub type BuildResult = Result<Vec<JoinHandle<Result<UrlPath, String>>>, String>;

pub trait Entry<'e> {
    fn name(&self) -> String;
    fn url(&self) -> UrlPath;
    fn build(&self, builder: &Builder<'e>) -> BuildResult;
    fn nav(&self) -> NavItem;
}

pub trait OutputEntry<'e>: Entry<'e> {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>);
}

pub trait ASTEntry<'e>: Entry<'e> {
    fn entity(&self) -> &Entity<'e>;
}

pub struct Builder<'e> {
    pub config: Arc<Config>,
    root: Namespace<'e>,
    file_roots: Vec<Root>,
    nav_cache: Option<String>,
}

impl<'e> Builder<'e> {
    pub fn new(config: Arc<Config>, root: Entity<'e>) -> Result<Self, String> {
        Ok(Self {
            config: config.clone(),
            root: Namespace::new(root),
            file_roots: Root::from_config(config),
            nav_cache: None,
        }
        .setup()?)
    }

    fn setup(mut self) -> Result<Self, String> {
        for script in self
            .config
            .scripts
            .css
            .iter()
            .chain(&self.config.scripts.js)
        {
            fs::write(
                self.config.output_dir.join(&script.name),
                script.content.as_ref(),
            )
            .map_err(|e| format!("Unable to copy {}: {e}", script.name))?;
        }
        self.prebuild()?;
        Ok(self)
    }

    pub fn create_output_for<E: OutputEntry<'e>>(&self, entry: &E) -> BuildResult {
        let (template, vars) = entry.output(self);
        Ok(vec![Self::create_output_in_thread(
            self.config.clone(),
            self.build_nav()?,
            entry.url(),
            template,
            vars,
        )])
    }

    fn create_output_in_thread(
        config: Arc<Config>,
        nav: String,
        target_url: UrlPath,
        template: Arc<String>,
        vars: Vec<(&'static str, Html)>,
    ) -> JoinHandle<Result<UrlPath, String>> {
        tokio::spawn(async move {
            let mut fmt = default_format(config.clone());
            fmt.extend(HashMap::from([(
                "page_url".to_owned(),
                target_url.to_absolute(config.clone()).to_string(),
            )]));
            fmt.extend(
                vars.into_iter()
                    .map(|(k, v)| (k.to_string(), v.gen_html()))
                    .collect::<Vec<_>>(),
            );

            let content = strfmt(&template, &fmt)
                .map_err(|e| format!("Unable to format {target_url}: {e}"))?;

            let page = strfmt(
                &config.templates.page,
                &HashMap::from([
                    (
                        "head_content".to_owned(),
                        strfmt(&config.templates.head, &fmt)
                            .map_err(|e| format!("Unable to format head for {target_url}: {e}"))?,
                    ),
                    ("navbar_content".to_owned(), nav),
                    ("main_content".to_owned(), content.clone()),
                ]),
            )
            .map_err(|e| format!("Unable to format {target_url}: {e}"))?;

            // Make sure output directory exists
            fs::create_dir_all(config.output_dir.join(target_url.to_pathbuf()))
                .map_err(|e| format!("Unable to create directory for {target_url}: {e}"))?;

            // Write the plain content output
            fs::write(
                &config
                    .output_dir
                    .join(target_url.to_pathbuf())
                    .join("content.html"),
                content,
            )
            .map_err(|e| format!("Unable to save {target_url}: {e}"))?;

            // Write the full page
            fs::write(
                &config
                    .output_dir
                    .join(target_url.to_pathbuf())
                    .join("index.html"),
                page,
            )
            .map_err(|e| format!("Unable to save {target_url}: {e}"))?;

            Ok(target_url)
        })
    }

    fn all_entries(&self) -> Vec<&dyn Entry<'e>> {
        self.root
            .entries
            .iter()
            .map(|p| p.1 as &dyn Entry<'e>)
            .chain(self.file_roots.iter().map(|p| p as &dyn Entry<'e>))
            .collect()
    }

    fn prebuild(&mut self) -> Result<(), String> {
        // Prebuild cached navbars for much faster docs builds
        self.prebuild_nav()?;

        Ok(())
    }

    pub async fn build(&self, pbar: Option<Arc<ProgressBar>>) -> Result<(), String> {
        let mut handles = Vec::new();

        // Spawn threads for creating docs for all entries
        for entry in self.all_entries() {
            handles.extend(entry.build(self)?);
        }

        if let Some(pbar) = pbar.clone() {
            pbar.set_message(format!("Generating output"));
        }

        futures::future::join_all(handles.into_iter().map(|handle| {
            let pbar = pbar.clone();
            tokio::spawn(async move {
                let res = handle.await.map_err(|e| format!("Unable to join {e}"))??;
                if let Some(pbar) = pbar {
                    pbar.set_message(format!("Built {res}"));
                }
                Result::<(), String>::Ok(())
            })
        }))
        .await
        .into_iter()
        .collect::<Result<Result<Vec<_>, _>, _>>()
        .map_err(|e| format!("Unable to join {e}"))??;

        // Create root index.html
        self.create_output_for(&Index {})?;

        Ok(())
    }

    pub fn build_nav(&self) -> Result<String, String> {
        if let Some(ref cached) = self.nav_cache {
            return Ok(cached.to_owned());
        }
        let mut fmt = default_format(self.config.clone());
        fmt.extend([
            (
                "entity_content".into(),
                self.root.nav().to_html(self.config.clone()).gen_html(),
            ),
            (
                "file_content".into(),
                self.file_roots
                    .iter()
                    .map(|root| root.nav().to_html(self.config.clone()).gen_html())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        ]);
        Ok(strfmt(&self.config.templates.nav, &fmt)
            .map_err(|e| format!("Unable to format navbar: {e}"))?)
    }

    fn prebuild_nav(&mut self) -> Result<(), String> {
        self.nav_cache = Some(self.build_nav()?);
        Ok(())
    }
}

fn default_format(config: Arc<Config>) -> HashMap<String, String> {
    HashMap::from([
        ("project_name".into(), config.project.name.clone()),
        ("project_version".into(), config.project.version.clone()),
        (
            "output_url".into(),
            config
                .output_url
                .as_ref()
                .unwrap_or(&UrlPath::new())
                .to_string(),
        ),
    ])
}

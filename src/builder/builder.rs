use clang::{Clang, Entity, EntityKind};
use indicatif::ProgressBar;
use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};
use strfmt::strfmt;
use tokio::task::JoinHandle;

use crate::{
    config::{Config, Source},
    html::{GenHtml, Html, HtmlElement, HtmlList, HtmlText},
    url::UrlPath,
};

use super::{files::Root, namespace::{Namespace, CppItemKind}, tutorial::TutorialFolder};

pub trait EntityMethods<'e> {
    /// Get the config source for this entity
    fn config_source(&self, config: Arc<Config>) -> Option<Arc<Source>>;

    /// Get the file where this entity is defined, if applicable
    fn definition_file(&self) -> Option<PathBuf>;

    /// Get a relative path to this file's header, if applicable
    fn header(&self, config: Arc<Config>) -> Option<PathBuf>;

    /// Get the relative for this entity
    fn rel_docs_url(&self) -> Option<UrlPath>;

    /// Get the full URL for this entity, valid for links
    fn abs_docs_url(&self, config: Arc<Config>) -> Option<UrlPath>;

    /// Get the full online URL of this entity
    fn github_url(&self, config: Arc<Config>) -> Option<String>;

    /// Get the include path for this entity
    fn include_path(&self, config: Arc<Config>) -> Option<UrlPath>;

    /// Get the fully qualified name for this entity
    fn full_name(&self) -> Vec<String>;

    /// Get the parents of this entity
    fn ancestorage(&self) -> Vec<Entity<'e>>;
}

impl<'e> EntityMethods<'e> for Entity<'e> {
    fn config_source(&self, config: Arc<Config>) -> Option<Arc<Source>> {
        // Get the definition header
        let path = self.header(config.clone())?;

        // Find the source that has this header
        config
            .sources
            .iter()
            .find(|src| path.starts_with(src.dir.to_pathbuf())).cloned()
    }

    fn definition_file(&self) -> Option<PathBuf> {
        self.get_definition()?
            .get_location()?
            .get_file_location()
            .file?
            .get_path()
            .into()
    }

    fn header(&self, config: Arc<Config>) -> Option<PathBuf> {
        let path = self.definition_file()?;
        path.strip_prefix(&config.input_dir)
            .unwrap_or(&path)
            .to_path_buf()
            .into()
    }

    fn rel_docs_url(&self) -> Option<UrlPath> {
        Some(
            CppItemKind::from(self)?
                .docs_category()
                .join(UrlPath::new_with_path(self.full_name()))
        )
    }

    fn abs_docs_url(&self, config: Arc<Config>) -> Option<UrlPath> {
        // If this is an std item, redirect to cppreference instead
        if self.full_name().first().is_some_and(|n| n == "std") {
            UrlPath::parse(&format!(
                "en.cppreference.com/w/cpp/{}/{}",
                self.definition_file()?.file_name()?.to_str()?,
                self.get_name()?
            ))
            .ok()
        } else {
            Some(self.rel_docs_url()?.to_absolute(config))
        }
    }

    fn github_url(&self, config: Arc<Config>) -> Option<String> {
        // If this is an std item, redirect to cppreference instead
        if self.full_name().first().is_some_and(|n| n == "std") {
            Some(format!(
                "https://en.cppreference.com/w/cpp/{}/{}",
                self.definition_file()?.file_name()?.to_str()?,
                self.get_name()?
            ))
        } else {
            Some(
                config.project.tree.clone()?
                    + &UrlPath::try_from(&self.header(config)?).ok()?.to_string(),
            )
        }
    }

    fn include_path(&self, config: Arc<Config>) -> Option<UrlPath> {
        UrlPath::try_from(&self.header(config.clone())?)
            .ok()?
            .strip_prefix(&self.config_source(config)?.dir)
            .into()
    }

    fn full_name(&self) -> Vec<String> {
        self.ancestorage()
            .iter()
            .map(|a| a.get_name().unwrap_or("_anon".into()))
            .collect()
    }

    fn ancestorage(&self) -> Vec<Entity<'e>> {
        let mut ancestors = Vec::new();
        if let Some(parent) = self.get_semantic_parent() {
            // apparently in github actions TranslationUnit enum doesn't 
            // match, so use this as a fail-safe
            if !parent.get_name().is_some_and(|p| p.ends_with(".cpp")) {
                match parent.get_kind() {
                    EntityKind::TranslationUnit
                    | EntityKind::UnexposedDecl
                    | EntityKind::UnexposedAttr
                    | EntityKind::UnexposedExpr
                    | EntityKind::UnexposedStmt => {}
                    _ => ancestors.extend(parent.ancestorage()),
                }
            }
        }
        ancestors.push(*self);
        ancestors
    }
}

pub enum NavItem {
    Root(Option<String>, Vec<NavItem>),
    Dir(String, Vec<NavItem>, Option<(String, bool)>, bool),
    Link(String, UrlPath, Option<(String, bool)>),
}

impl NavItem {
    pub fn new_link(name: &str, url: UrlPath, icon: Option<(&str, bool)>) -> NavItem {
        NavItem::Link(name.into(), url, icon.map(|s| (s.0.into(), s.1)))
    }

    pub fn new_dir(name: &str, items: Vec<NavItem>, icon: Option<(&str, bool)>) -> NavItem {
        NavItem::Dir(name.into(), items, icon.map(|s| (s.0.into(), s.1)), false)
    }

    pub fn new_dir_open(
        name: &str,
        items: Vec<NavItem>,
        icon: Option<(&str, bool)>,
        open: bool,
    ) -> NavItem {
        NavItem::Dir(name.into(), items, icon.map(|s| (s.0.into(), s.1)), open)
    }

    pub fn new_root(name: Option<&str>, items: Vec<NavItem>) -> NavItem {
        NavItem::Root(name.map(|s| s.into()), items)
    }

    pub fn to_html(&self, config: Arc<Config>) -> Html {
        match self {
            NavItem::Link(name, url, icon) => HtmlElement::new("a")
                .with_attr(
                    "onclick",
                    format!("return navigate('{}')", url.to_absolute(config.clone())),
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

            NavItem::Dir(name, items, icon, open) => HtmlElement::new("details")
                .with_attr_opt("open", open.then_some(""))
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
    fn output(&self, builder: &'e Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>);
    fn title(&self, builder: &'e Builder<'e>) -> String;
    fn description(&self, builder: &'e Builder<'e>) -> String;
}

pub trait ASTEntry<'e>: Entry<'e> {
    fn entity(&self) -> &Entity<'e>;
    fn category(&self) -> &'static str;
    fn output_title(&self, builder: &'e Builder<'e>) -> String{
        format!(
            "{} Docs in {}",
            self.name(),
            builder.config.project.name
        )
    }
    fn output_description(&self, builder: &'e Builder<'e>) -> String {
        format!(
            "Documentation for the {} {} in {}",
            self.name(),
            self.category(),
            builder.config.project.name
        )
    }
}

pub struct Builder<'e> {
    pub config: Arc<Config>,
    pub root: Namespace<'e>,
    pub clang: &'e Clang,
    pub index: &'e clang::Index<'e>,
    pub args: &'e [String],
    file_roots: Vec<Root>,
    tutorials: TutorialFolder,
    nav_cache: Option<String>,
}

impl<'e> Builder<'e> {
    pub fn new(
        config: Arc<Config>,
        root: Entity<'e>,
        clang: &'e Clang,
        index: &'e clang::Index<'e>,
        args: &'e [String],
    ) -> Result<Self, String> {
        Self {
            config: config.clone(),
            root: Namespace::new_root(root),
            clang,
            index,
            args,
            file_roots: Root::from_config(config.clone()),
            tutorials: TutorialFolder::from_config(config),
            nav_cache: None,
        }
        .setup()
    }

    fn setup(mut self) -> Result<Self, String> {
        // copy scripts
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
        // copy icon
        if let Some(ref icon) = self.config.project.icon {
            fs::copy(
                self.config.input_dir.join(icon),
                self.config.output_dir.join("icon.png"),
            )
            .map_err(|e| format!("Unable to copy icon: {e}"))?;
        }
        // copy tutorial assets
        if let Some(ref tutorials) = self.config.tutorials {
            for asset in &tutorials.assets {
                let output = self.config.output_dir.join(
                    // if the tutorials are in docs and the assets are in 
                    // docs/assets, then they are probably referenced with 
                    // just assets/image.png so we should strip the docs 
                    // part
                    asset.strip_prefix(&tutorials.dir).unwrap_or(asset)
                );
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(self.config.output_dir.join(parent))
                    .map_err(|e| format!(
                        "Unable to create asset directory '{}': {e}",
                        output.to_string_lossy()
                    ))?;
                }
                fs::copy(self.config.input_dir.join(asset), output)
                .map_err(|e| format!(
                    "Unable to copy asset '{}': {e}, {}",
                    asset.to_string_lossy(),
                    self.config.input_dir.join(asset).to_string_lossy(),
                ))?;
            }
        }
        // prebuild nav for performance
        self.prebuild()?;
        Ok(self)
    }

    pub fn create_output_for<E: OutputEntry<'e>>(&'e self, entry: &E) -> BuildResult {
        let (template, vars) = entry.output(self);
        Ok(vec![Self::create_output_in_thread(
            self.config.clone(),
            self.build_nav()?,
            entry.title(self),
            entry.description(self),
            entry.url(),
            template,
            vars,
        )])
    }

    fn create_output_in_thread(
        config: Arc<Config>,
        nav: String,
        title: String,
        description: String,
        target_url: UrlPath,
        template: Arc<String>,
        vars: Vec<(&'static str, Html)>,
    ) -> JoinHandle<Result<UrlPath, String>> {
        tokio::spawn(async move {
            let mut fmt = default_format(config.clone());
            fmt.extend(HashMap::from([
                (
                    "page_url".to_owned(),
                    target_url.to_absolute(config.clone()).to_string(),
                ),
                ("page_title".to_owned(), title),
                ("page_description".to_owned(), description),
            ]));
            fmt.extend(
                vars.into_iter()
                    .map(|(k, v)| (k.to_string(), v.gen_html()))
                    .collect::<Vec<_>>(),
            );

            let content = strfmt(&template, &fmt)
                .map_err(|e| format!("Unable to format {target_url}: {e}"))?;

            let mut page_fmt = default_format(config.clone());
            page_fmt.extend(HashMap::from([
                (
                    "head_content".to_owned(),
                    strfmt(&config.templates.head, &fmt)
                        .map_err(|e| format!("Unable to format head for {target_url}: {e}"))?,
                ),
                ("navbar_content".to_owned(), nav),
                ("main_content".to_owned(), content.clone()),
            ]));
            let page = strfmt(&config.templates.page, &page_fmt)
                .map_err(|e| format!("Unable to format {target_url}: {e}"))?;

            // Make sure output directory exists
            fs::create_dir_all(config.output_dir.join(target_url.to_pathbuf()))
                .map_err(|e| format!("Unable to create directory for {target_url}: {e}"))?;

            // Write the plain content output
            fs::write(
                config
                    .output_dir
                    .join(target_url.to_pathbuf())
                    .join("content.html"),
                content,
            )
            .map_err(|e| format!("Unable to save {target_url}: {e}"))?;

            // Write the full page
            fs::write(
                config
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
            .chain([&self.tutorials as &dyn Entry])
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
            pbar.set_message("Generating output".to_string());
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

        Ok(())
    }

    pub fn build_nav(&self) -> Result<String, String> {
        if let Some(ref cached) = self.nav_cache {
            return Ok(cached.to_owned());
        }
        let mut fmt = default_format(self.config.clone());
        fmt.extend([
            (
                "tutorial_content".into(),
                self.tutorials.nav().to_html(self.config.clone()).gen_html(),
            ),
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
        strfmt(&self.config.templates.nav, &fmt)
            .map_err(|e| format!("Unable to format navbar: {e}"))
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
            "project_repository".into(),
            config.project.repository.clone().unwrap_or(String::new()),
        ),
        (
            "project_icon".into(),
            config
                .project
                .icon
                .as_ref()
                .and(Some(format!(
                    "<img src=\"{}/icon.png\">",
                    config
                        .output_url
                        .as_ref()
                        .unwrap_or(&UrlPath::new())
                )))
                .unwrap_or(String::new()),
        ),
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

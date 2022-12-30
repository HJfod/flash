use clang::{Entity, EntityKind};
use indicatif::ProgressBar;
use std::{collections::HashMap, fs, sync::Arc};
use strfmt::strfmt;
use tokio::task::JoinHandle;

use crate::{config::Config, url::UrlPath};

use super::{files::Root, index::Index, namespace::Namespace};

pub trait EntityMethods<'e> {
    fn rel_url(&self) -> UrlPath;
    fn docs_url(&self, config: Arc<Config>) -> UrlPath {
        self.rel_url().to_absolute(config)
    }
}

impl<'e> EntityMethods<'e> for Entity<'e> {
    fn rel_url(&self) -> UrlPath {
        UrlPath::new_with_path(get_fully_qualified_name(&self))
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

    pub fn to_html(&self, config: Arc<Config>) -> String {
        match self {
            NavItem::Link(name, url, icon) => format!(
                "<a onclick='return navigate(\"{0}\")' href='{0}'>{1}{2}</a>",
                url.to_absolute(config),
                icon.as_ref()
                    .map(|i| format!(
                        "<i data-feather='{}' class='icon {}'></i>",
                        i.0,
                        if i.1 { "variant" } else { "" }
                    ))
                    .unwrap_or(String::new()),
                sanitize_html(name),
            ),

            NavItem::Dir(name, items, icon) => format!(
                "<details>
                    <summary>
                        <i data-feather='chevron-right'></i>
                        {}{}
                    </summary>
                    <div>{}</div>
                </details>",
                icon.as_ref()
                    .map(|i| format!(
                        "<i data-feather='{}' class='icon {}'></i>",
                        i.0,
                        if i.1 { "variant" } else { "" }
                    ))
                    .unwrap_or(String::new()),
                sanitize_html(name),
                items
                    .iter()
                    .map(|i| i.to_html(config.clone()))
                    .collect::<String>()
            ),

            NavItem::Root(name, items) => {
                if let Some(name) = name {
                    format!(
                        "<details open class='root'>
                        <summary>
                            <i data-feather='chevron-right'></i>
                            {}
                        </summary>
                        <div>{}</div>
                    </details>",
                        sanitize_html(name),
                        items
                            .iter()
                            .map(|i| i.to_html(config.clone()))
                            .collect::<String>()
                    )
                } else {
                    items
                        .iter()
                        .map(|i| i.to_html(config.clone()))
                        .collect::<String>()
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
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, String)>);
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
        vars: Vec<(&'static str, String)>,
    ) -> JoinHandle<Result<UrlPath, String>> {
        tokio::spawn(async move {
            let mut fmt = default_format(config.clone());
            fmt.extend(HashMap::from([(
                "page_url".to_owned(),
                target_url.to_absolute(config.clone()).to_string(),
            )]));
            fmt.extend(
                vars.iter()
                    .map(|(k, v)| (k.to_string(), v.to_owned()))
                    .collect::<Vec<_>>(),
            );

            let content = strfmt(&template, &fmt)
                .map_err(|e| format!("Unable to format {target_url}: {e}"))?;

            let page = strfmt(
                &config.templates.page,
                &HashMap::from([
                    (
                        "head_content".to_owned(),
                        strfmt(&config.templates.head, &default_format(config.clone()))
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
                self.root.nav().to_html(self.config.clone()),
            ),
            (
                "file_content".into(),
                self.file_roots
                    .iter()
                    .map(|root| root.nav().to_html(self.config.clone()))
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

pub fn get_fully_qualified_name(entity: &Entity) -> Vec<String> {
    get_ancestorage(entity)
        .iter()
        .map(|a| a.get_name().unwrap_or("_anon".into()))
        .collect()
}

pub fn get_ancestorage<'e>(entity: &Entity<'e>) -> Vec<Entity<'e>> {
    let mut ancestors = Vec::new();
    if let Some(parent) = entity.get_semantic_parent() {
        match parent.get_kind() {
            EntityKind::TranslationUnit
            | EntityKind::UnexposedDecl
            | EntityKind::UnexposedAttr
            | EntityKind::UnexposedExpr
            | EntityKind::UnexposedStmt => {}
            _ => ancestors.extend(get_ancestorage(&parent)),
        }
    }
    ancestors.push(entity.clone());
    ancestors
}

pub fn get_github_url(config: Arc<Config>, entity: &Entity) -> Option<String> {
    let path = entity
        .get_definition()?
        .get_location()?
        .get_file_location()
        .file?
        .get_path();

    Some(
        config.docs.tree.clone()?
            + &UrlPath::try_from(
                &path
                    .strip_prefix(&config.input_dir)
                    .unwrap_or(&path)
                    .to_path_buf(),
            )
            .ok()?
            .to_string(),
    )
}

pub fn get_header_path(config: Arc<Config>, entity: &Entity) -> Option<UrlPath> {
    let path = entity
        .get_definition()?
        .get_location()?
        .get_file_location()
        .file?
        .get_path();

    let rel_path = path.strip_prefix(&config.input_dir).unwrap_or(&path);

    for root in &config.browser.roots {
        if let Ok(stripped) = rel_path.strip_prefix(root.path.to_pathbuf()) {
            return Some(
                root.include_prefix
                    .join(UrlPath::try_from(&stripped.to_path_buf()).ok()?),
            );
        }
    }

    None
}

pub fn sanitize_html(html: &str) -> String {
    html.replace("<", "&lt;").replace(">", "&gt;")
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

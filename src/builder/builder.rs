use clang::{Entity, EntityKind};
use indicatif::ProgressBar;
use std::{collections::HashMap, fs};
use strfmt::strfmt;

use crate::{config::Config, url::UrlPath};

use super::{namespace::Namespace, files::Root, index::Index};

pub enum NavItem {
    Root(Option<String>, Vec<NavItem>),
    Dir(String, Vec<NavItem>, Option<String>),
    Link(String, UrlPath, Option<String>),
}

impl NavItem {
    pub fn new_link(name: &str, url: UrlPath, icon: Option<&str>) -> NavItem {
        NavItem::Link(name.into(), url, icon.map(|s| s.into()))
    }

    pub fn new_dir(name: &str, items: Vec<NavItem>, icon: Option<&str>) -> NavItem {
        NavItem::Dir(name.into(), items, icon.map(|s| s.into()))
    }

    pub fn new_root(name: Option<&str>, items: Vec<NavItem>) -> NavItem {
        NavItem::Root(name.map(|s| s.into()), items)
    }

    pub fn to_html(&self, config: &Config) -> String {
        match self {
            NavItem::Link(name, url, icon) => format!(
                "<a onclick='return navigate(\"{0}\")' href='{0}'>{1}{2}</a>",
                url.to_absolute(config),
                icon
                    .as_ref()
                    .map(|i| format!("<i data-feather='{}' class='icon'></i>", i))
                    .unwrap_or(String::new()),
                name
            ),

            NavItem::Dir(name, items, icon) => format!(
                "<details>
                    <summary>
                        <i data-feather='chevron-right'></i>
                        {}{}
                    </summary>
                    <div>{}</div>
                </details>",
                icon
                    .as_ref()
                    .map(|i| format!("<i data-feather='{}' class='icon'></i>", i))
                    .unwrap_or(String::new()),
                name,
                items.iter().map(|i| i.to_html(config)).collect::<String>()
            ),

            NavItem::Root(name, items) => if let Some(name) = name {
                format!(
                    "<details open class='root'>
                        <summary>
                            <i data-feather='chevron-right'></i>
                            {}
                        </summary>
                        <div>{}</div>
                    </details>",
                    name, items.iter().map(|i| i.to_html(config)).collect::<String>()
                )
            } else {
                items.iter().map(|i| i.to_html(config)).collect::<String>()
            }
        }
    }
}

pub trait AnEntry<'e> {
    fn name(&self) -> String;
    fn url(&self) -> UrlPath;
    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String>;
    fn nav(&self) -> NavItem;
}

pub trait OutputEntry<'c, 'e>: AnEntry<'e> {
    fn output(&self, builder: &Builder<'c, 'e>) -> (&'c String, Vec<(&str, String)>);
}

pub struct Builder<'c, 'e> {
    pub config: &'c Config,
    root: Namespace<'e>,
    file_roots: Vec<Root<'c>>,
    nav_cache: Option<String>,
}

impl<'c, 'e> Builder<'c, 'e> {
    pub fn new(config: &'c Config, root: Entity<'e>) -> Self {
        Self {
            config,
            root: Namespace::new(root),
            file_roots: Root::from_config(config),
            nav_cache: None,
        }.setup()
    }

    fn setup(self) -> Self {
        for script in self.config.scripts.css.iter().chain(&self.config.scripts.js) {
            fs::write(
                self.config.output_dir.join(&script.name),
                &script.content
            ).unwrap();
        }
        self
    }

    pub fn create_output_for<E: OutputEntry<'c, 'e>>(&self, entry: &E) -> Result<(), String> {
        let (template, vars) = entry.output(self);
        let target_url = &entry.url();
        
        let mut fmt = default_format(self.config);
        fmt.extend(HashMap::from([
            ("page_url".to_owned(), target_url.to_absolute(self.config).to_string()),
        ]));
        fmt.extend(vars.iter().map(|(k, v)| (k.to_string(), v.to_owned())).collect::<Vec<_>>());
    
        let content = strfmt(&template, &fmt)
            .map_err(|e| format!("Unable to format {target_url}: {e}"))?;
        
        let page = strfmt(
            &self.config.templates.page,
            &HashMap::from([
                (
                    "head_content".to_owned(), 
                    strfmt(
                        &self.config.templates.head,
                        &default_format(self.config)
                    ).map_err(|e| format!("Unable to format head for {target_url}: {e}"))?
                ),
                ("navbar_content".to_owned(), self.build_nav()?),
                ("main_content".to_owned(), content.clone()),
            ])
        )
            .map_err(|e| format!("Unable to format {target_url}: {e}"))?;

        // Make sure output directory exists
        fs::create_dir_all(self.config.output_dir.join(target_url.to_pathbuf()))
            .map_err(|e| format!("Unable to create directory for {target_url}: {e}"))?;

        // Write the plain content output
        fs::write(&self.config.output_dir.join(target_url.to_pathbuf()).join("content.html"), content)
            .map_err(|e| format!("Unable to save {target_url}: {e}"))?;

        // Write the full page
        fs::write(&self.config.output_dir.join(target_url.to_pathbuf()).join("index.html"), page)
            .map_err(|e| format!("Unable to save {target_url}: {e}"))?;
    
        Ok(())
    }

    fn all_entries(&self) -> Vec<&dyn AnEntry<'e>> {
        self.root.entries
            .iter()
            .map(|p| p.1 as &dyn AnEntry<'e>)
            .chain(self.file_roots.iter().map(|p| p as &dyn AnEntry<'e>))
            .collect()
    }

    pub fn build(&mut self, pbar: Option<&ProgressBar>) -> Result<(), String> {
        // For tracking progress
        let entries_len = self.root.entries.len();
        let total_len = (entries_len + self.file_roots.len()) as f64;

        // Prebuild cached navbars for much faster docs builds
        self.prebuild_nav()?;

        // Create docs for all entries
        let mut i = 0f64;
        for entry in self.all_entries() {
            if let Some(pbar) = pbar {
                pbar.set_position((i / total_len * pbar.length().unwrap_or(1) as f64) as u64);
            }
            i += 1f64;
            entry.build(self)?;
        }

        // Create root index.html
        self.create_output_for(&Index {})?;
    
        Ok(())
    }

    pub fn build_nav(&self) -> Result<String, String> {
        if let Some(ref cached) = self.nav_cache {
            return Ok(cached.to_owned());
        }
        let mut fmt = default_format(self.config);
        fmt.extend([
            ("entity_content".into(), self.root.nav().to_html(self.config)),
            (
                "file_content".into(),
                self.file_roots
                    .iter()
                    .map(|root| root.nav().to_html(self.config))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        ]);
        Ok(
            strfmt(&self.config.templates.nav, &fmt)
                .map_err(|e| format!("Unable to format navbar: {e}"))?
        )
    }

    fn prebuild_nav(&mut self) -> Result<(), String> {
        self.nav_cache = Some(self.build_nav()?);
        Ok(())
    }
}

pub fn get_fully_qualified_name(entity: &Entity) -> Vec<String> {
    let mut name = Vec::new();
    if let Some(parent) = entity.get_semantic_parent() {
        if !matches!(parent.get_kind(), EntityKind::TranslationUnit) {
            name.extend(get_fully_qualified_name(&parent));
        }
    }
    name.push(entity.get_name().unwrap_or("_anon".into()));
    name
}

pub fn get_ancestorage<'e>(entity: &Entity<'e>) -> Vec<Entity<'e>> {
    let mut ancestors = Vec::new();
    if let Some(parent) = entity.get_semantic_parent() {
        if !matches!(parent.get_kind(), EntityKind::TranslationUnit) {
            ancestors.extend(get_ancestorage(&parent));
        }
    }
    ancestors.push(entity.clone());
    ancestors
}

pub fn get_github_url(config: &Config, entity: &Entity) -> Option<String> {
    let path = entity
        .get_definition()?
        .get_location()?
        .get_file_location()
        .file?
        .get_path();

    Some(
        config.docs.tree.clone()? + 
            &UrlPath::try_from(
                &path
                    .strip_prefix(&config.input_dir)
                    .unwrap_or(&path)
                    .to_path_buf()
            ).ok()?.to_string(),
    )
}

pub fn get_header_path(config: &Config, entity: &Entity) -> Option<UrlPath> {
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
                root.include_prefix.join(UrlPath::try_from(&stripped.to_path_buf()).ok()?)
            );
        }
    }

    None
}

fn default_format(config: &Config) -> HashMap<String, String> {
    HashMap::from([
        ("project_name".into(), config.project.name.clone()),
        ("project_version".into(), config.project.version.clone()),
        ("output_url".into(), config.output_url.as_ref().unwrap_or(&UrlPath::new()).to_string()),
    ])
}

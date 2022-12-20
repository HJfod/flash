use clang::{Entity, EntityKind};
use indicatif::ProgressBar;
use std::{collections::HashMap, fs, path::PathBuf};
use strfmt::strfmt;

use crate::config::Config;

pub trait AnEntry<'e> {
    fn new(entity: Entity<'e>) -> Self;
    fn entity(&self) -> &Entity<'e>;
    fn name(&self) -> String;
    fn url(&self) -> String {
        String::from("./") + &get_fully_qualified_name(&self.entity()).join("/")
    }
    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String>;
    fn build_nav(&self, relative: &String) -> String;
}

pub enum Entry<'e> {
    Namespace(Namespace<'e>),
    Class(Class<'e>),
}

impl<'e> Entry<'e> {
    pub fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        match self {
            Entry::Namespace(ns) => ns.build(builder),
            Entry::Class(cs) => cs.build(builder),
        }
    }

    pub fn build_nav(&self, relative: &String) -> String {
        match self {
            Entry::Namespace(ns) => ns.build_nav(relative),
            Entry::Class(cs) => cs.build_nav(relative),
        }
    }
}

pub struct Namespace<'e> {
    entity: Entity<'e>,
    entries: HashMap<String, Entry<'e>>,
}

impl<'e> AnEntry<'e> for Namespace<'e> {
    fn new(entity: Entity<'e>) -> Self {
        let mut ret = Self {
            entity,
            entries: HashMap::new(),
        };
        ret.load_entries();
        ret
    }

    fn entity(&self) -> &Entity<'e> {
        &self.entity
    }

    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        for (_, entry) in &self.entries {
            entry.build(builder)?;
        }
        Ok(())
    }

    fn build_nav(&self, relative: &String) -> String {
        let mut namespaces = self.entries
            .iter()
            .filter(|e| matches!(e.1, Entry::Namespace(_)))
            .collect::<Vec<_>>();
        
        namespaces.sort_by_key(|p| p.0);

        let mut other = self.entries
            .iter()
            .filter(|e| !matches!(e.1, Entry::Namespace(_)))
            .collect::<Vec<_>>();

        other.sort_by_key(|p| p.0);

        namespaces.extend(other);

        // If this is a translation unit (aka root in Builder) then just output 
        // the contents of the nav section and not the title
        if matches!(self.entity.get_kind(), EntityKind::TranslationUnit) {
            namespaces
                .iter()
                .map(|e| e.1.build_nav(relative))
                .collect::<Vec<_>>()
                .join("\n")
        }
        // Otherwise foldable namespace name
        else {
            format!(
                "<details>
                    <summary><i data-feather='chevron-right'></i>{}</summary>
                    <div>{}</div>
                </details>
                ",
                self.name(),
                namespaces
                    .iter()
                    .map(|e| e.1.build_nav(relative))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    }

    fn name(&self) -> String {
        self.entity.get_name().unwrap_or("<Anonymous namespace>".into())
    }
}

impl<'e> Namespace<'e> {
    fn load_entries(&mut self) {
        for child in &self.entity.get_children() {
            if child.is_in_system_header() || child.get_name().is_none() {
                continue;
            }
            match child.get_kind() {
                EntityKind::Namespace => {
                    let entry = Namespace::new(child.clone());
                    // Merge existing entries of namespace
                    if let Some(key) = self.entries.get_mut(&entry.name()) {
                        if let Entry::Namespace(ns) = key {
                            ns.entries.extend(entry.entries);
                        }
                    }
                    // Insert new namespace
                    else {
                        self.entries.insert(entry.name(), Entry::Namespace(entry));
                    }
                },

                EntityKind::StructDecl | EntityKind::ClassDecl => {
                    if child.is_definition() {
                        let entry = Class::new(child.clone());
                        self.entries.insert(entry.name(), Entry::Class(entry));
                    }
                },

                _ => continue,
            }
        }
    }
}

pub struct Class<'e> {
    entity: Entity<'e>,
}

impl<'e> AnEntry<'e> for Class<'e> {
    fn new(entity: Entity<'e>) -> Self {
        Self {
            entity
        }
    }

    fn entity(&self) -> &Entity<'e> {
        &self.entity
    }

    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        // Target directory
        let dir_path = builder.config.output_dir.join(&self.url());
        fs::create_dir_all(&dir_path).unwrap();
    
        write_docs_output(
            builder,
            &builder.config.presentation.class_template,
            &self.url(),
            [
                ("name".to_string(), self.entity.get_name().unwrap()),
                (
                    "description".into(),
                    self.entity
                        .get_parsed_comment()
                        .map(|c| c.as_html())
                        .unwrap_or("<p>No Description Provided</p>".into()),
                ),
                (
                    "header_link".into(),
                    get_header_url(builder.config, &self.entity)
                        .map(|url| format!("<a href='{}'>View Header</a>", url))
                        .unwrap_or(String::new()),
                ),
            ]
        )?;
    
        Ok(())
    }

    fn build_nav(&self, relative: &String) -> String {
        format!(
            "<a href='.{}/{}'>{}</a>\n",
            "/..".repeat(relative.matches("/").count()),
            get_fully_qualified_name(&self.entity).join("/"),
            self.name()
        )
    }

    fn name(&self) -> String {
        self.entity.get_name().unwrap_or("<Anonymous class or struct>".into())
    }
}

pub struct Builder<'c, 'e> {
    pub config: &'c Config,
    pub root: Namespace<'e>,
}

impl<'c, 'e> Builder<'c, 'e> {
    pub fn new(config: &'c Config, root: Entity<'e>) -> Self {
        Self {
            config,
            root: Namespace::new(root),
        }
    }

    pub fn build(&mut self, pbar: Option<&ProgressBar>) -> Result<(), String> {
        let mut i = 0f64;
        let len = self.root.entries.len() as f64;
        for (_, entry) in &self.root.entries {
            if let Some(pbar) = pbar {
                pbar.set_position((i / len * pbar.length().unwrap_or(1) as f64) as u64);
            }
            i += 1f64;
            entry.build(self)?;
        }
    
        write_docs_output(
            &self,
            &self.config.presentation.index_template,
            &String::new(),
            []
        )?;
    
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

pub fn get_header_url(config: &Config, entity: &Entity) -> Option<String> {
    let path = entity
        .get_definition()?
        .get_location()?
        .get_file_location()
        .file?
        .get_path();

    Some(
        config.docs.tree.clone()?
            + "/"
            + &path
                .strip_prefix(&config.input_dir)
                .unwrap_or(&path)
                .to_str()?
                .replace("\\", "/"),
    )
}

pub fn get_css_path(config: &Config) -> PathBuf {
    config.output_dir.join("style.css")
}

fn default_format(config: &Config, target_url: &String) -> HashMap<String, String> {
    HashMap::from([
        ("project_name".into(), config.project.name.clone()),
        ("project_version".into(), config.project.version.clone()),
        (
            "style_css_url".into(), 
            format!("./{}style.css", "../".repeat(target_url.matches("/").count()))
        ),
        ("default_script".into(), config.presentation.js.clone()),
    ])
}

fn write_docs_output<'e, T: IntoIterator<Item = (String, String)>>(
    builder: &Builder<'_, 'e>,
    template: &String,
    target_url: &String,
    vars: T
) -> Result<(), String> {
    let mut fmt = default_format(builder.config, &target_url);
    fmt.extend(vars);
    fmt.extend([
        (
            "default_head".into(),
            strfmt(
                &builder.config.presentation.head_template,
                &default_format(builder.config, target_url)
            ).map_err(|e| format!("Unable to format head for {target_url}: {e}"))?
        ),
        ("default_navbar".into(), builder.root.build_nav(target_url)),
    ]);

    let data = strfmt(template, &fmt)
        .map_err(|e| format!("Unable to format {target_url}: {e}"))?;

    fs::write(&builder.config.output_dir.join(target_url).join("index.html"), data)
        .map_err(|e| format!("Unable to save {target_url}: {e}"))?;

    Ok(())
}

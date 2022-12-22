use clang::{Entity, EntityKind};
use indicatif::ProgressBar;
use std::{collections::HashMap, fs, path::PathBuf};
use strfmt::strfmt;

use crate::config::Config;

use super::{namespace::Namespace, files::Root, index::Index};

pub trait AnEntry<'e> {
    fn name(&self) -> String;
    fn url(&self) -> String;
    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String>;
    fn build_nav(&self, relative: &String) -> String;
}

pub trait OutputEntry<'c, 'e>: AnEntry<'e> {
    fn output(&self, builder: &Builder<'c, 'e>) -> (&'c String, Vec<(String, String)>);
}

pub struct Builder<'c, 'e> {
    pub config: &'c Config,
    pub root: Namespace<'e>,
    pub file_roots: Vec<Root<'c>>,
}

impl<'c, 'e> Builder<'c, 'e> {
    pub fn new(config: &'c Config, root: Entity<'e>) -> Self {
        Self {
            config,
            root: Namespace::new(root),
            file_roots: Root::from_config(config),
        }
    }

    pub fn create_output_for<E: OutputEntry<'c, 'e>>(&self, entry: &E) -> Result<(), String> {
        let (template, vars) = entry.output(self);
        let target_url = &entry.url();
        
        let mut fmt = default_format(self.config, target_url);
        fmt.extend(vars);
        fmt.extend([
            (
                "default_head".into(),
                strfmt(
                    &self.config.presentation.head_template,
                    &default_format(self.config, target_url)
                ).map_err(|e| format!("Unable to format head for {target_url}: {e}"))?
            ),
            ("navbar".into(), self.build_nav(target_url)?),
        ]);
    
        let data = strfmt(&template, &fmt)
            .map_err(|e| format!("Unable to format {target_url}: {e}"))?;
    
        fs::create_dir_all(self.config.output_dir.join(target_url))
            .map_err(|e| format!("Unable to create directory for {target_url}: {e}"))?;

        fs::write(&self.config.output_dir.join(target_url).join("index.html"), data)
            .map_err(|e| format!("Unable to save {target_url}: {e}"))?;
    
        Ok(())
    }

    pub fn build(&mut self, pbar: Option<&ProgressBar>) -> Result<(), String> {
        let entries_len = self.root.entries.len();
        let total_len = (entries_len + self.file_roots
            .iter()
            .map(|p| p.dir.dirs.len() + p.dir.files.len())
            .reduce(|acc, p| acc + p)
            .unwrap_or(0)
        ) as f64;

        let mut i = 0f64;
        for (_, entry) in &self.root.entries {
            if let Some(pbar) = pbar {
                pbar.set_position((i / total_len * pbar.length().unwrap_or(1) as f64) as u64);
            }
            i += 1f64;
            entry.build(self)?;
        }

        for root in &self.file_roots {
            if let Some(pbar) = pbar {
                pbar.set_position((i / total_len * pbar.length().unwrap_or(1) as f64) as u64);
            }
            i += 1f64;
            root.build(self)?;
        }
    
        self.create_output_for(&Index {})?;
    
        Ok(())
    }

    pub fn build_nav(&self, relative: &String) -> Result<String, String> {
        let mut fmt = default_format(self.config, &relative);
        fmt.extend([
            ("entity_content".into(), self.root.build_nav(relative)),
            (
                "file_content".into(),
                self.file_roots
                    .iter()
                    .map(|root| root.build_nav(relative))
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        ]);
        Ok(
            strfmt(&self.config.presentation.nav_template, &fmt)
                .map_err(|e| format!("Unable to format navbar: {e}"))?
        )
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

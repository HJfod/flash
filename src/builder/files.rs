use super::builder::{BuildResult, Builder, Entry, NavItem, OutputEntry};
use crate::{
    config::{BrowserRoot, Config},
    url::UrlPath, html::html::{Html, HtmlText},
};
use std::{collections::HashMap, path::Path, sync::Arc};

pub struct File {
    def: Arc<BrowserRoot>,
    path: UrlPath,
    prefix: UrlPath,
}

impl<'e> Entry<'e> for File {
    fn name(&self) -> String {
        self.path.raw_file_name().unwrap().clone()
    }

    fn url(&self) -> UrlPath {
        UrlPath::parse("files").unwrap().join(&self.path)
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), Some(("file", false)))
    }
}

impl<'e> OutputEntry<'e> for File {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        (
            builder.config.templates.file.clone(),
            vec![
                ("name", HtmlText::new(self.name()).into()),
                ("description", Html::p("No Description Provided")),
                (
                    "file_url",
                    HtmlText::new(
                        builder
                        .config
                        .docs
                        .tree
                        .as_ref()
                        .map(|tree| tree.to_owned() + &self.def.path.join(&self.path).to_string())
                        .unwrap_or("".into())
                    )
                    .into(),
                ),
                ("file_path", HtmlText::new(self.prefix.join(&self.path).to_raw_string()).into()),
            ],
        )
    }
}

impl File {
    pub fn new(def: Arc<BrowserRoot>, path: UrlPath, prefix: UrlPath) -> Self {
        Self { def, path, prefix }
    }
}

pub struct Dir {
    def: Arc<BrowserRoot>,
    path: UrlPath,
    prefix: UrlPath,
    pub dirs: HashMap<String, Dir>,
    pub files: HashMap<String, File>,
}

impl<'b, 'e> Entry<'e> for Dir {
    fn name(&self) -> String {
        self.path.raw_file_name().unwrap().to_owned()
    }

    fn url(&self) -> UrlPath {
        UrlPath::parse("files").unwrap().join(&self.path)
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        let mut handles = Vec::new();
        for (_, dir) in &self.dirs {
            handles.extend(dir.build(builder)?);
        }
        for (_, file) in &self.files {
            handles.extend(file.build(builder)?);
        }
        Ok(handles)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_dir(
            &self.name(),
            self.dirs
                .iter()
                .map(|e| e.1.nav())
                .chain(self.files.iter().map(|e| e.1.nav()))
                .collect::<Vec<_>>(),
            Some(("folder", false)),
        )
    }
}

impl Dir {
    pub fn new(def: Arc<BrowserRoot>, path: UrlPath, prefix: UrlPath) -> Self {
        Self {
            def,
            path,
            prefix,
            dirs: HashMap::new(),
            files: HashMap::new(),
        }
    }
}

pub struct Root {
    pub def: Arc<BrowserRoot>,
    pub dir: Dir,
}

impl<'b, 'e> Entry<'e> for Root {
    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        self.dir.build(builder)
    }

    fn name(&self) -> String {
        self.def.name.clone()
    }

    fn url(&self) -> UrlPath {
        UrlPath::new()
    }

    fn nav(&self) -> NavItem {
        NavItem::Root(
            Some(self.name()),
            self.dir
                .dirs
                .iter()
                .map(|e| e.1.nav())
                .chain(self.dir.files.iter().map(|e| e.1.nav()))
                .collect(),
        )
    }
}

impl Root {
    pub fn from_config(config: Arc<Config>) -> Vec<Self> {
        let mut roots = config
            .browser
            .roots
            .iter()
            .map(|root| Root {
                def: root.clone(),
                dir: Dir::new(
                    root.clone(),
                    root.name.clone().try_into().unwrap(),
                    root.include_prefix.clone(),
                ),
            })
            .collect::<Vec<_>>();

        for file in config.filtered_includes() {
            // Figure out which root(s) this file belongs to (if any), and add to it
            for root in &mut roots {
                let Ok(cut_path) = file.strip_prefix(root.def.path.to_pathbuf()) else {
                    continue;
                };

                // If this is a directory, just add the whole structure
                if file.is_dir() {
                    root.add_dirs(cut_path);
                } else {
                    // Add to parent if one exists, or to root if one doesn't
                    let prefix = root.def.include_prefix.clone();
                    let url = UrlPath::try_from(&cut_path.to_path_buf()).unwrap();
                    let def = root.def.clone();
                    root.try_add_dirs(cut_path.parent()).files.insert(
                        url.file_name().unwrap().to_owned(),
                        File::new(def, url, prefix.clone()),
                    );
                }
            }
        }

        roots
    }

    pub fn add_dirs(&mut self, path: &Path) -> &mut Dir {
        let mut target = &mut self.dir;
        for part in path {
            let part_name = part.to_str().unwrap().to_string();
            let url = target.url();
            target = target.dirs.entry(part_name.clone()).or_insert(Dir::new(
                self.def.clone(),
                url.join(UrlPath::try_from(&part_name).unwrap()),
                self.def.include_prefix.clone(),
            ));
        }
        target
    }

    pub fn try_add_dirs(&mut self, path: Option<&Path>) -> &mut Dir {
        if let Some(path) = path {
            self.add_dirs(path)
        } else {
            &mut self.dir
        }
    }
}

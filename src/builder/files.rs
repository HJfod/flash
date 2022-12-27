
use std::{collections::HashMap, path::Path};
use crate::{config::{BrowserRoot, Config}, url::UrlPath};
use super::builder::{AnEntry, Builder, OutputEntry, NavItem};

pub struct File<'b> {
    def: &'b BrowserRoot,
    path: UrlPath,
    prefix: UrlPath,
}

impl<'b, 'e> AnEntry<'e> for File<'b> {
    fn name(&self) -> String {
        self.path.file_name().unwrap().clone()
    }

    fn url(&self) -> UrlPath {
        UrlPath::parse("files").unwrap().join(&self.path)
    }

    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), Some("file"))
    }
}

impl<'b, 'c, 'e> OutputEntry<'c, 'e> for File<'b> {
    fn output(&self, builder: &Builder<'c, 'e>) -> (&'c String, Vec<(&str, String)>) {
        (
            &builder.config.templates.file,
            vec![
                ("name", self.name()),
                ("description", "<p>No Description Provided</p>".into()),
                (
                    "file_url",
                    builder.config.docs.tree.as_ref().map(|tree| 
                        tree.to_owned() + &self.def.path.join(&self.path).to_string()
                    ).unwrap_or("".into()),
                ),
                ("file_path", self.prefix.join(&self.path).to_raw_string()),
            ]
        )
    }
}

impl<'b> File<'b> {
    pub fn new(def: &'b BrowserRoot, path: UrlPath, prefix: UrlPath) -> Self {
        Self {
            def,
            path,
            prefix,
        }
    }
}

pub struct Dir<'b> {
    def: &'b BrowserRoot,
    path: UrlPath,
    prefix: UrlPath,
    pub dirs: HashMap<String, Dir<'b>>,
    pub files: HashMap<String, File<'b>>,
}

impl<'b, 'e> AnEntry<'e> for Dir<'b> {
    fn name(&self) -> String {
        self.path.file_name().unwrap().to_owned()
    }

    fn url(&self) -> UrlPath {
        UrlPath::parse("files").unwrap().join(&self.path)
    }

    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        for (_, dir) in &self.dirs {
            dir.build(builder)?;
        }
        for (_, file) in &self.files {
            file.build(builder)?;
        }
        Ok(())
    }

    fn nav(&self) -> NavItem {
        NavItem::new_dir(
            &self.name(),
            self.dirs
                .iter()
                .map(|e| e.1.nav())
                .chain(self.files.iter().map(|e| e.1.nav()))
                .collect::<Vec<_>>(),
            Some("folder"),
        )
    }
}

impl<'b> Dir<'b> {
    pub fn new(def: &'b BrowserRoot, path: UrlPath, prefix: UrlPath) -> Self {
        Self {
            def,
            path,
            prefix,
            dirs: HashMap::new(),
            files: HashMap::new(),
        }
    }
}

pub struct Root<'b> {
    pub def: &'b BrowserRoot,
    pub dir: Dir<'b>,
}

impl<'b, 'e> AnEntry<'e> for Root<'b> {
    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
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
            self.dir.dirs
                .iter()
                .map(|e| e.1.nav())
                .chain(self.dir.files.iter().map(|e| e.1.nav()))
                .collect()
        )
    }
}

impl<'b> Root<'b> {
    pub fn from_config(config: &'b Config) -> Vec<Self> {
        let mut roots = config.browser.roots.iter().map(|root| Root {
            def: root,
            dir: Dir::new(root, root.name.clone().try_into().unwrap(), root.include_prefix.clone()),
        }).collect::<Vec<_>>();
    
        for file in config.filtered_includes() {
            // Figure out which root(s) this file belongs to (if any), and add to it
            for root in &mut roots {
                let Ok(cut_path) = file.strip_prefix(root.def.path.to_pathbuf()) else {
                    continue;
                };
    
                // If this is a directory, just add the whole structure
                if file.is_dir() {
                    root.add_dirs(cut_path);
                }
                else {
                    // Add to parent if one exists, or to root if one doesn't
                    let prefix = root.def.include_prefix.clone();
                    let url = UrlPath::try_from(&cut_path.to_path_buf()).unwrap();
                    let def = root.def;
                    root.try_add_dirs(cut_path.parent()).files.insert(
                        url.file_name().unwrap().to_owned(),
                        File::new(def, url, prefix.clone())
                    );
                }
            }
        }
    
        roots
    }

    pub fn add_dirs(&mut self, path: &Path) -> &mut Dir<'b> {
        let mut target = &mut self.dir;
        for part in path {
            let part_name = part.to_str().unwrap().to_string();
            let url = target.url();
            target = target.dirs
                .entry(part_name.clone())
                .or_insert(Dir::new(
                    self.def,
                    url.join(UrlPath::try_from(&part_name).unwrap()),
                    self.def.include_prefix.clone()
                ));
        }
        target
    }

    pub fn try_add_dirs(&mut self, path: Option<&Path>) -> &mut Dir<'b> {
        if let Some(path) = path {
            self.add_dirs(path)
        }
        else {
            &mut self.dir
        }
    }
}

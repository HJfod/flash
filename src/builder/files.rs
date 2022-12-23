
use std::{collections::HashMap, path::{PathBuf, Path}};
use crate::config::{BrowserRoot, Config};
use super::builder::{AnEntry, Builder, OutputEntry, NavItem};

pub struct File {
    name: String,
    path: String,
}

impl<'e> AnEntry<'e> for File {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn url(&self) -> String {
        String::from("./") + &self.path
    }

    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name, &self.path, Some("file"))
    }
}

impl<'c, 'e> OutputEntry<'c, 'e> for File {
    fn output(&self, builder: &Builder<'c, 'e>) -> (&'c String, Vec<(String, String)>) {
        (
            &builder.config.templates.file,
            vec![
                ("name".to_string(), self.name.clone()),
                (
                    "description".into(),
                    "<p>No Description Provided</p>".into(),
                ),
                (
                    "file_link".into(),
                    builder.config.docs.tree.as_ref().map(|tree| 
                        format!("<a href='{}/{}'>View Header</a>", tree, self.path)
                    ).unwrap_or("".into()),
                ),
            ]
        )
    }
}

impl File {
    pub fn new(path: PathBuf) -> Self {
        Self {
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            path: String::from("files/") + &path.components()
                .map(|c| c.as_os_str().to_str().unwrap().to_string())
                .collect::<Vec<_>>().join("/")
        }
    }
}

pub struct Dir {
    pub name: String,
    pub path: String,
    pub dirs: HashMap<String, Dir>,
    pub files: HashMap<String, File>,
}

impl<'e> AnEntry<'e> for Dir {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn url(&self) -> String {
        String::from("./files/") + &self.path
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
            &self.name,
            self.dirs
                .iter()
                .map(|e| e.1.nav())
                .chain(self.files.iter().map(|e| e.1.nav()))
                .collect::<Vec<_>>(),
            Some("folder"),
        )
    }
}

impl Dir {
    pub fn new(path: PathBuf) -> Self {
        Self {
            name: path.file_name().unwrap().to_str().unwrap().to_string(),
            path: String::from("files/") + &path.components()
                .map(|c| c.as_os_str().to_str().unwrap().to_string())
                .collect::<Vec<_>>().join("/"),
            dirs: HashMap::new(),
            files: HashMap::new(),
        }
    }
}

pub struct Root<'b> {
    pub def: &'b BrowserRoot,
    pub dir: Dir,
}

impl<'b, 'e> AnEntry<'e> for Root<'b> {
    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        self.dir.build(builder)
    }

    fn name(&self) -> String {
        self.def.name.clone()
    }

    fn url(&self) -> String {
        ".".into()
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
            dir: Dir::new(root.name.clone().into()),
        }).collect::<Vec<_>>();
    
        for file in config.filtered_includes() {
            // Figure out which root(s) this file belongs to (if any), and add to it
            for root in &mut roots {
                let Ok(cut_path) = file.strip_prefix(&root.def.path) else {
                    continue;
                };
    
                // If this is a directory, just add the whole structure
                if file.is_dir() {
                    root.add_dirs(cut_path);
                }
                else {
                    // Add to parent if one exists, or to root if one doesn't
                    root.try_add_dirs(cut_path.parent()).files.insert(
                        cut_path.file_name().unwrap().to_str().unwrap().to_string(),
                        File::new(cut_path.to_owned())
                    );
                }
            }
        }
    
        roots
    }

    pub fn add_dirs(&mut self, path: &Path) -> &mut Dir {
        let mut target = &mut self.dir;
        for part in path {
            let url = target.url();
            target = target.dirs
                .entry(part.to_str().unwrap().to_string())
                .or_insert(Dir::new(PathBuf::from(url).join(part)));
        }
        target
    }

    pub fn try_add_dirs(&mut self, path: Option<&Path>) -> &mut Dir {
        if let Some(path) = path {
            self.add_dirs(path)
        }
        else {
            &mut self.dir
        }
    }
}

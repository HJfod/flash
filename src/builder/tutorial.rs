use crate::{
    config::Config,
    html::{Html, HtmlElement},
    url::UrlPath,
};
use std::{collections::HashMap, ffi::OsStr, fs, path::PathBuf, sync::Arc, cmp::Ordering};

use super::{
    traits::{BuildResult, Entry, NavItem, OutputEntry},
    builder::Builder,
    shared::fmt_section,
    markdown::{extract_metadata_from_md, output_tutorial, Metadata},
};

pub struct Tutorial {
    path: UrlPath,
    metadata: Metadata,
    unparsed_content: String,
}

impl Tutorial {
    pub fn new(config: Arc<Config>, path: UrlPath) -> Self {
        let unparsed_content = fs::read_to_string(
            config
                .input_dir
                .join(&config.tutorials.as_ref().unwrap().dir)
                .join(path.to_pathbuf()),
        )
        .unwrap_or_else(|_| panic!("Unable to read tutorial {}", path.to_raw_string()));

        Self {
            metadata: extract_metadata_from_md(
                &unparsed_content,
                path.remove_extension(".md").raw_file_name()
            ).unwrap(),
            unparsed_content,
            path,
        }
    }
}

impl<'e> Entry<'e> for Tutorial {
    fn name(&self) -> String {
        self.metadata.title.clone().unwrap()
    }

    fn url(&self) -> UrlPath {
        self.path.remove_extension(".md")
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(
            self.metadata.title.as_ref().unwrap(),
            self.url(),
            Some(
                self.metadata.icon.as_ref()
                    .map(|i| (i.as_str(), false))
                    .unwrap_or(("bookmark", false))
            ),
            Vec::new(),
        )
    }
}

impl<'e> OutputEntry<'e> for Tutorial {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        (
            builder.config.templates.tutorial.clone(),
            output_tutorial(
                self,
                builder,
                &self.unparsed_content,
                Html::Raw(String::new())
            )
        )
    }

    fn description(&self, builder: &'e Builder<'e>) -> String {
        self.metadata.description.clone().unwrap_or(format!(
            "Tutorial for {}",
            builder.config.project.name
        ))
    }
}

pub struct TutorialFolder {
    is_root: bool,
    is_open: bool,
    path: UrlPath,
    metadata: Option<Metadata>,
    index: Option<String>,
    folders: HashMap<String, TutorialFolder>,
    tutorials: HashMap<String, Tutorial>,
}

impl TutorialFolder {
    fn from_folder(config: Arc<Config>, path: &PathBuf, depth: i32) -> Option<Self> {
        let mut folders = HashMap::new();
        let mut tutorials = HashMap::new();

        let stripped_path = path
            .strip_prefix(
                &config
                    .input_dir
                    .join(&config.tutorials.as_ref().unwrap().dir),
            )
            .unwrap_or(path)
            .to_path_buf();

        // find tutorials (markdown files)
        for file in fs::read_dir(path).ok()? {
            let Ok(file) = file else { continue; };
            let Ok(ty) = file.file_type() else { continue; };
            let path = file.path();

            // if this is a directory, add it only if it has tutorials
            if ty.is_dir() {
                if let Some(folder) =
                    TutorialFolder::from_folder(config.clone(), &file.path(), depth + 1)
                {
                    folders.insert(folder.name(), folder);
                }
            }
            // markdown files are tutorials
            else if ty.is_file() && path.extension() == Some(OsStr::new("md")) &&
                // skip special files
                match path.file_name().map(|f| f.to_string_lossy().to_lowercase()) {
                    Some(val) => !matches!(val.as_str(), "readme.md" | "index.md"),
                    None => false,
                }
            {
                let stripped_path = path
                    .strip_prefix(
                        &config
                            .input_dir
                            .join(&config.tutorials.as_ref().unwrap().dir),
                    )
                    .unwrap_or(&path)
                    .to_path_buf();

                let Ok(url) = UrlPath::try_from(&stripped_path) else { continue; };
                let tut = Tutorial::new(config.clone(), url);
                tutorials.insert(tut.name(), tut);
            }
        }

        let index = if path.join("index.md").exists() {
            fs::read_to_string(path.join("index.md")).ok()
        } else {
            None
        };

        // only consider this a tutorial folder if it has some tutorials
        (!folders.is_empty() || !tutorials.is_empty()).then_some(Self {
            is_root: false,
            is_open: depth < 2,
            path: UrlPath::try_from(&stripped_path).ok()?,
            metadata: index.as_ref().and_then(|i| extract_metadata_from_md(i, None)),
            index,
            folders,
            tutorials,
        })
    }

    pub fn from_config(config: Arc<Config>) -> Self {
        if let Some(ref tutorials) = config.tutorials &&
            let Some(mut res) = Self::from_folder(
                config.clone(), &config.input_dir.join(&tutorials.dir), 0
            )
        {
            res.is_root = true;
            res
        }
        else {
            Self {
                is_root: true,
                is_open: true,
                path: UrlPath::new(),
                metadata: None,
                index: None,
                folders: HashMap::new(),
                tutorials: HashMap::new(),
            }
        }
    }

    pub fn folders_sorted(&self) -> Vec<&TutorialFolder> {
        let mut vec = self.folders.iter().collect::<Vec<_>>();
        vec.sort_by_key(|t| t.0);
        vec.into_iter().map(|(_, v)| v).collect()
    }

    pub fn tutorials_sorted(&self) -> Vec<&Tutorial> {
        let mut vec = self.tutorials.iter().collect::<Vec<_>>();
        vec.sort_unstable_by(|a, b| {
            match (a.1.metadata.order, b.1.metadata.order) {
                (Some(a), Some(b)) => a.cmp(&b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => a.0.cmp(&b.0),
            }
        });
        vec.into_iter().map(|(_, v)| v).collect()
    }
}

impl<'e> Entry<'e> for TutorialFolder {
    fn name(&self) -> String {
        self.metadata
            .clone()
            .and_then(|m| m.title)
            .unwrap_or(self.path.raw_file_name().unwrap_or(String::from("")))
    }

    fn url(&self) -> UrlPath {
        if self.is_root {
            UrlPath::new()
        } else {
            self.path.clone()
        }
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        let mut handles = Vec::new();
        handles.extend(builder.create_output_for(self)?);
        for dir in self.folders.values() {
            handles.extend(dir.build(builder)?);
        }
        for file in self.tutorials.values() {
            handles.extend(file.build(builder)?);
        }
        Ok(handles)
    }

    fn nav(&self) -> NavItem {
        if self.is_root {
            NavItem::new_root(
                None,
                self.tutorials_sorted()
                    .into_iter()
                    .map(|e| e.nav())
                    .chain(self.folders_sorted().iter().map(|e| e.nav()))
                    .collect::<Vec<_>>(),
            )
        } else {
            NavItem::new_dir_open(
                &self.name(),
                self.tutorials_sorted()
                    .into_iter()
                    .map(|e| e.nav())
                    .chain(self.folders_sorted().iter().map(|e| e.nav()))
                    .collect::<Vec<_>>(),
                self.metadata.as_ref()
                    .and_then(|m| m.icon.as_ref())
                    .map(|i| (i.as_str(), false)),
                self.is_open,
            )
        }
    }
}

impl<'e> OutputEntry<'e> for TutorialFolder {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        (
            if self.index.is_some() {
                builder.config.templates.tutorial.clone()
            } else {
                builder.config.templates.tutorial_index.clone()
            },
            output_tutorial(
                self,
                builder,
                self.index.as_ref().map(|s| s.as_str()).unwrap_or(""),
                fmt_section(
                    "Pages",
                    self.tutorials_sorted()
                        .iter()
                        .map(|tut| {
                            HtmlElement::new("ul")
                                .with_child(HtmlElement::new("li").with_child(
                                    HtmlElement::new("a")
                                        .with_text(&tut.name())
                                        .with_attr(
                                            "href",
                                            tut.url().to_absolute(builder.config.clone()),
                                        ),
                                ))
                                .into()
                        })
                        .collect(),
                )
            )
        )
    }

    fn description(&self, builder: &'e Builder<'e>) -> String {
        if self.is_root {
            format!("Documentation for {}", builder.config.project.name)
        }
        else {
            self.metadata
                .clone()
                .and_then(|m| m.description)
                .unwrap_or(format!(
                    "Tutorials for {}",
                    builder.config.project.name
                ))
        }
    }
}

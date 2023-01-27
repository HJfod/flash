use crate::{
    config::Config,
    html::{Html, HtmlElement, HtmlText},
    url::UrlPath,
};
use std::{collections::HashMap, ffi::OsStr, fs, path::PathBuf, sync::Arc};

use super::{
    builder::{BuildResult, Builder, Entry, NavItem, OutputEntry},
    shared::{extract_title_from_md, fmt_markdown, fmt_section},
};

pub struct Tutorial {
    path: UrlPath,
    title: String,
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
            title: extract_title_from_md(&unparsed_content)
                .unwrap_or(path.raw_file_name().unwrap()),
            unparsed_content,
            path,
        }
    }
}

impl<'e> Entry<'e> for Tutorial {
    fn name(&self) -> String {
        self.path.raw_file_name().unwrap().replace(".md", "")
    }

    fn url(&self) -> UrlPath {
        UrlPath::parse("tutorials").unwrap().join(&self.path.remove_extension(".md"))
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.title, self.url(), Some(("bookmark", false)))
    }
}

impl<'e> OutputEntry<'e> for Tutorial {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        (
            builder.config.templates.tutorial.clone(),
            vec![
                ("title", HtmlText::new(self.name()).into()),
                ("content", fmt_markdown(
                    builder.config.clone(),
                    Some(UrlPath::part("tutorials")),
                    &self.unparsed_content
                )),
                ("links", Html::Raw(String::new())),
            ],
        )
    }
}

pub struct TutorialFolder {
    is_root: bool,
    is_open: bool,
    path: UrlPath,
    title: Option<String>,
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
            title: index.as_ref().and_then(extract_title_from_md),
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
                title: None,
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
        vec.sort_by_key(|t| t.0);
        vec.into_iter().map(|(_, v)| v).collect()
    }
}

impl<'e> Entry<'e> for TutorialFolder {
    fn name(&self) -> String {
        self.title
            .clone()
            .unwrap_or(self.path.raw_file_name().unwrap_or(String::from("_")))
    }

    fn url(&self) -> UrlPath {
        if self.is_root {
            UrlPath::new()
        } else {
            UrlPath::parse("tutorials").unwrap().join(&self.path)
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
                    .iter()
                    .map(|e| e.nav())
                    .chain(self.folders_sorted().iter().map(|e| e.nav()))
                    .collect::<Vec<_>>(),
            )
        } else {
            NavItem::new_dir_open(
                &self.name(),
                self.tutorials_sorted()
                    .iter()
                    .map(|e| e.nav())
                    .chain(self.folders_sorted().iter().map(|e| e.nav()))
                    .collect::<Vec<_>>(),
                None,
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
            vec![
                ("title", HtmlText::new(self.name()).into()),
                (
                    "content",
                    self.index
                        .as_ref()
                        .map(|i| fmt_markdown(
                            builder.config.clone(),
                            Some(UrlPath::part("tutorials")),
                            i
                        ))
                        .unwrap_or(Html::p("")),
                ),
                (
                    "links",
                    fmt_section(
                        "Pages",
                        self.tutorials_sorted()
                            .iter()
                            .map(|tut| {
                                HtmlElement::new("ul")
                                    .with_child(HtmlElement::new("li").with_child(
                                        HtmlElement::new("a").with_text(&tut.title).with_attr(
                                            "href",
                                            tut.url().to_absolute(builder.config.clone()),
                                        ),
                                    ))
                                    .into()
                            })
                            .collect(),
                    ),
                ),
            ],
        )
    }
}

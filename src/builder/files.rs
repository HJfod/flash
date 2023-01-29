
use super::{
    builder::Builder,
    traits::{BuildResult, Entry, NavItem, OutputEntry, ASTEntry},
    shared::{fmt_fun_decl, fmt_section, fmt_classlike_decl},
    namespace::CppItemKind
};
use crate::{
    config::{Config, Source},
    html::{Html, HtmlText},
    url::UrlPath,
};
use std::{collections::HashMap, path::Path, sync::Arc};

pub struct File {
    source: Arc<Source>,
    path: UrlPath,
}

impl File {
    pub fn new(def: Arc<Source>, path: UrlPath) -> Self {
        Self { source: def, path }
    }
}

impl<'e> Entry<'e> for File {
    fn name(&self) -> String {
        self.path.raw_file_name().unwrap()
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
    fn output(&self, builder: &'e Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        let matcher = |entry: &dyn ASTEntry<'e>| -> bool {
            entry.entity().get_location()
                .and_then(|file| file.get_file_location().file)
                .is_some_and(|file|
                    file.get_path() == builder.config.input_dir.join(
                        self.source.dir.join(&self.path).to_raw_string()
                    )
                )
        };

        (
            builder.config.templates.file.clone(),
            vec![
                ("name", HtmlText::new(self.name()).into()),
                ("description", Html::p("")),
                (
                    "file_url",
                    HtmlText::new(
                        builder
                            .config
                            .project
                            .tree
                            .as_ref()
                            .map(|tree| {
                                tree.to_owned() + &self.source.dir.join(&self.path).to_string()
                            })
                            .unwrap_or("".into()),
                    )
                    .into(),
                ),
                (
                    "file_path",
                    HtmlText::new(self.source.dir.join(&self.path).to_raw_string()).into(),
                ),
                (
                    "functions",
                    fmt_section(
                        "Functions",
                        builder.root
                            .get(&|entry| 
                                matches!(
                                    CppItemKind::from(entry.entity()),
                                    Some(CppItemKind::Function)
                                ) && matcher(entry)
                            )
                            .into_iter()
                            .map(|fun| fmt_fun_decl(fun.entity(), builder))
                            .collect()
                    ),
                ),
                (
                    "classes",
                    fmt_section(
                        "Classes",
                        builder.root
                            .get(&|entry| 
                                matches!(
                                    CppItemKind::from(entry.entity()),
                                    Some(CppItemKind::Class)
                                ) && matcher(entry)
                            )
                            .into_iter()
                            .map(|cls| fmt_classlike_decl(cls.entity(), "class", builder))
                            .collect()
                    ),
                ),
                (
                    "structs",
                    fmt_section(
                        "Structs",
                        builder.root
                            .get(&|entry| 
                                matches!(
                                    CppItemKind::from(entry.entity()),
                                    Some(CppItemKind::Struct)
                                ) && matcher(entry)
                            )
                            .into_iter()
                            .map(|cls| fmt_classlike_decl(cls.entity(), "struct", builder))
                            .collect()
                    ),
                ),
            ],
        )
    }

    fn title(&self, builder: &'e Builder<'e>) -> String {
        format!(
            "{} Docs in {}",
            self.name(),
            builder.config.project.name
        )
    }

    fn description(&self, builder: &'e Builder<'e>) -> String {
        format!(
            "Documentation for {} in {}",
            self.path,
            builder.config.project.name
        )
    }
}

pub struct Dir {
    source: Arc<Source>,
    path: UrlPath,
    pub dirs: HashMap<String, Dir>,
    pub files: HashMap<String, File>,
}

impl Dir {
    pub fn new(def: Arc<Source>, path: UrlPath) -> Self {
        Self {
            source: def,
            path,
            dirs: HashMap::new(),
            files: HashMap::new(),
        }
    }
}

impl<'e> Entry<'e> for Dir {
    fn name(&self) -> String {
        self.path.raw_file_name().unwrap()
    }

    fn url(&self) -> UrlPath {
        UrlPath::parse("files").unwrap().join(&self.path)
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        let mut handles = Vec::new();
        for dir in self.dirs.values() {
            handles.extend(dir.build(builder)?);
        }
        for file in self.files.values() {
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

pub struct Root {
    pub source: Arc<Source>,
    pub dir: Dir,
}

impl Root {
    pub fn from_config(config: Arc<Config>) -> Vec<Self> {
        let mut roots = config
            .sources
            .iter()
            .map(|root| Root {
                source: root.clone(),
                dir: Dir::new(root.clone(), root.name.clone().try_into().unwrap()),
            })
            .collect::<Vec<_>>();

        for root in &mut roots {
            for file in root.source.include.clone() {
                let Ok(cut_path) = file.strip_prefix(root.source.dir.to_pathbuf()) else {
                    continue;
                };

                // If this is a directory, just add the whole structure
                if file.is_dir() {
                    root.add_dirs(cut_path);
                } else {
                    // Add to parent if one exists, or to root if one doesn't
                    let url = UrlPath::try_from(&cut_path.to_path_buf()).unwrap();
                    let def = root.source.clone();
                    root.try_add_dirs(cut_path.parent())
                        .files
                        .insert(url.file_name().unwrap().to_owned(), File::new(def, url));
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
                self.source.clone(),
                url.join(UrlPath::try_from(&part_name).unwrap()),
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

impl<'e> Entry<'e> for Root {
    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        self.dir.build(builder)
    }

    fn name(&self) -> String {
        self.source.name.clone()
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

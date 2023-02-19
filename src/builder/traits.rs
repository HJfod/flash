use clang::{Entity, EntityKind, Accessibility};

use std::{path::PathBuf, sync::Arc, collections::HashMap};

use tokio::task::JoinHandle;

use crate::{
    config::{Config, Source},
    html::{Html, HtmlElement, HtmlList, HtmlText},
    url::UrlPath,
};

use super::{namespace::CppItemKind, builder::Builder, shared::member_fun_link};

pub trait EntityMethods<'e> {
    /// Get the config source for this entity
    fn config_source(&self, config: Arc<Config>) -> Option<Arc<Source>>;

    /// Get the file where this entity is defined, if applicable
    fn definition_file(&self) -> Option<PathBuf>;

    /// Get a relative path to this file's header, if applicable
    fn header(&self, config: Arc<Config>) -> Option<PathBuf>;

    /// Get the relative for this entity
    fn rel_docs_url(&self) -> Option<UrlPath>;

    /// Get the full URL for this entity, valid for links
    fn abs_docs_url(&self, config: Arc<Config>) -> Option<UrlPath>;

    /// Get the full online URL of this entity
    fn github_url(&self, config: Arc<Config>) -> Option<String>;

    /// Get the include path for this entity
    fn include_path(&self, config: Arc<Config>) -> Option<UrlPath>;

    /// Get the fully qualified name for this entity
    fn full_name(&self) -> Vec<String>;

    /// Get the parents of this entity
    fn ancestorage(&self) -> Vec<Entity<'e>>;
}

impl<'e> EntityMethods<'e> for Entity<'e> {
    fn config_source(&self, config: Arc<Config>) -> Option<Arc<Source>> {
        // Get the definition header
        let path = self.header(config.clone())?;

        // Find the source that has this header
        config
            .sources
            .iter()
            .find(|src| path.starts_with(src.dir.to_pathbuf())).cloned()
    }

    fn definition_file(&self) -> Option<PathBuf> {
        self.get_definition()?
            .get_location()?
            .get_file_location()
            .file?
            .get_path()
            .into()
    }

    fn header(&self, config: Arc<Config>) -> Option<PathBuf> {
        let path = self.definition_file()?;
        path.strip_prefix(&config.input_dir)
            .unwrap_or(&path)
            .to_path_buf()
            .into()
    }

    fn rel_docs_url(&self) -> Option<UrlPath> {
        Some(
            CppItemKind::from(self)?
                .docs_category()
                .join(UrlPath::new_with_path(self.full_name()))
        )
    }

    fn abs_docs_url(&self, config: Arc<Config>) -> Option<UrlPath> {
        // If this is an std item, redirect to cppreference instead
        if self.full_name().first().is_some_and(|n| n == "std") {
            UrlPath::parse(&format!(
                "en.cppreference.com/w/cpp/{}/{}",
                self.definition_file()?.file_name()?.to_str()?,
                self.get_name()?
            ))
            .ok()
        } else {
            Some(self.rel_docs_url()?.to_absolute(config))
        }
    }

    fn github_url(&self, config: Arc<Config>) -> Option<String> {
        // If this is an std item, redirect to cppreference instead
        if self.full_name().first().is_some_and(|n| n == "std") {
            Some(format!(
                "https://en.cppreference.com/w/cpp/{}/{}",
                self.definition_file()?.file_name()?.to_str()?,
                self.get_name()?
            ))
        } else {
            Some(
                config.project.tree.clone()?
                    + &UrlPath::try_from(&self.header(config)?).ok()?.to_string(),
            )
        }
    }

    fn include_path(&self, config: Arc<Config>) -> Option<UrlPath> {
        UrlPath::try_from(&self.header(config.clone())?)
            .ok()?
            .strip_prefix(&self.config_source(config)?.dir)
            .into()
    }

    fn full_name(&self) -> Vec<String> {
        self.ancestorage()
            .iter()
            .map(|a| a.get_name().unwrap_or("_anon".into()))
            .collect()
    }

    fn ancestorage(&self) -> Vec<Entity<'e>> {
        let mut ancestors = Vec::new();
        if let Some(parent) = self.get_semantic_parent() {
            // apparently in github actions TranslationUnit enum doesn't 
            // match, so use this as a fail-safe
            if !parent.get_name().is_some_and(|p| p.ends_with(".cpp")) {
                match parent.get_kind() {
                    EntityKind::TranslationUnit
                    | EntityKind::UnexposedDecl
                    | EntityKind::UnexposedAttr
                    | EntityKind::UnexposedExpr
                    | EntityKind::UnexposedStmt => {}
                    _ => ancestors.extend(parent.ancestorage()),
                }
            }
        }
        ancestors.push(*self);
        ancestors
    }
}

#[derive(Clone)]
pub struct SubItem {
    pub title: String,
    pub heading: String,
    pub icon: Option<(String, bool)>,
}

impl SubItem {
    pub fn for_classlike(entity: &Entity) -> Vec<SubItem> {
        let Some(kind) = CppItemKind::from(entity) else {
            return Vec::new();
        };
        match kind {
            CppItemKind::Class | CppItemKind::Struct => {
                get_member_functions(entity, Access::All, Include::All)
                    .into_iter()
                    .filter_map(|e| Some(SubItem {
                        title: e.get_name()?,
                        heading: member_fun_link(&e)?,
                        icon: Some((String::from("code"), true)),
                    }))
                    .collect()
            }

            CppItemKind::Namespace | CppItemKind::Function => Vec::new()
        }
    }
}

pub enum NavItem {
    Root(Option<String>, Vec<NavItem>),
    Dir(String, Vec<NavItem>, Option<(String, bool)>, bool),
    Link(String, UrlPath, Option<(String, bool)>, Vec<SubItem>),
}

impl NavItem {
    pub fn new_link(
        name: &str,
        url: UrlPath,
        icon: Option<(&str, bool)>,
        suboptions: Vec<SubItem>,
    ) -> NavItem {
        NavItem::Link(name.into(), url, icon.map(|s| (s.0.into(), s.1)), suboptions)
    }

    pub fn new_dir(name: &str, items: Vec<NavItem>, icon: Option<(&str, bool)>) -> NavItem {
        NavItem::Dir(name.into(), items, icon.map(|s| (s.0.into(), s.1)), false)
    }

    pub fn new_dir_open(
        name: &str,
        items: Vec<NavItem>,
        icon: Option<(&str, bool)>,
        open: bool,
    ) -> NavItem {
        NavItem::Dir(name.into(), items, icon.map(|s| (s.0.into(), s.1)), open)
    }

    pub fn new_root(name: Option<&str>, items: Vec<NavItem>) -> NavItem {
        NavItem::Root(name.map(|s| s.into()), items)
    }

    pub fn suboptions_titles(&self, config: Arc<Config>) -> HashMap<String, usize> {
        match self {
            NavItem::Link(name, _, _, suboptions) => {
                let mut res = HashMap::new();
                for opt in suboptions.iter().map(|o| format!("{}::{}", name, o.title)) {
                    if let Some(r) = res.get_mut(&opt) {
                        *r += 1;
                    }
                    else {
                        res.insert(opt, 0);
                    }
                }
                res
            },

            NavItem::Dir(name, items, _, _) => items.iter()
                .flat_map(|i| i.suboptions_titles(config.clone()))
                .into_iter()
                .map(|(t, count)| (format!("{}::{}", name, t), count))
                .collect(),
            
            NavItem::Root(_, items) => items.iter()
                .flat_map(|i| i.suboptions_titles(config.clone()))
                .collect()
        }
    }

    pub fn to_html(&self, config: Arc<Config>) -> Html {
        match self {
            NavItem::Link(name, url, icon, _) => {
                HtmlList::new(vec![
                    HtmlElement::new("a")
                        .with_attr(
                            "onclick",
                            format!("return navigate('{}')", url.to_absolute(config.clone())),
                        )
                        .with_attr("href", url.to_absolute(config.clone()))
                        .with_child_opt(icon.as_ref().map(|i| {
                            HtmlElement::new("i")
                                .with_attr("data-feather", &i.0)
                                .with_class("icon")
                                .with_class_opt(i.1.then_some("variant"))
                        }))
                        .with_child(HtmlText::new(name))
                        .into()
                ]).into()
            }

            NavItem::Dir(name, items, icon, open) => HtmlElement::new("details")
                .with_attr_opt("open", open.then_some(""))
                .with_child(
                    HtmlElement::new("summary")
                        .with_child(
                            HtmlElement::new("i").with_attr("data-feather", "chevron-right"),
                        )
                        .with_child_opt(icon.as_ref().map(|i| {
                            HtmlElement::new("i")
                                .with_attr("data-feather", &i.0)
                                .with_class("icon")
                                .with_class_opt(i.1.then_some("variant"))
                        }))
                        .with_child(HtmlText::new(name)),
                )
                .with_child(
                    HtmlElement::new("div")
                        .with_children(items.iter().map(|i| i.to_html(config.clone())).collect()),
                )
                .into(),

            NavItem::Root(name, items) => {
                if let Some(name) = name {
                    HtmlElement::new("details")
                        .with_attr("open", "")
                        .with_attr("class", "root")
                        .with_child(
                            HtmlElement::new("summary")
                                .with_child(
                                    HtmlElement::new("i")
                                        .with_attr("data-feather", "chevron-right"),
                                )
                                .with_child(HtmlText::new(name)),
                        )
                        .with_child(HtmlElement::new("div").with_children(
                            items.iter().map(|i| i.to_html(config.clone())).collect(),
                        ))
                        .into()
                } else {
                    HtmlList::new(items.iter().map(|i| i.to_html(config.clone())).collect()).into()
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
    fn output(&self, builder: &'e Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>);
    fn description(&self, builder: &'e Builder<'e>) -> String;
}

pub trait ASTEntry<'e>: Entry<'e> {
    fn entity(&self) -> &Entity<'e>;
    fn category(&self) -> &'static str;
    fn output_description(&self, builder: &'e Builder<'e>) -> String {
        format!(
            "Documentation for the {} {} in {}",
            self.name(),
            self.category(),
            builder.config.project.name
        )
    }
}

pub enum Access {
    All,
    Public,
    Protected,
}

pub enum Include {
    All,
    Members,
    Statics,
}

pub fn get_member_functions<'e>(
    entity: &Entity<'e>,
    visibility: Access,
    include_statics: Include,
) -> Vec<Entity<'e>> {
    entity
        .get_children()
        .into_iter()
        .filter(|child| {
            child.get_kind() == EntityKind::Method
                && match include_statics {
                    Include::Members => !child.is_static_method(),
                    Include::Statics => child.is_static_method(),
                    Include::All => true,
                }
                && match child.get_accessibility() {
                    Some(Accessibility::Protected)
                    => matches!(visibility, Access::All | Access::Protected),
                    Some(Accessibility::Public)
                    => matches!(visibility, Access::All | Access::Public),
                    _ => false,
                }
        })
        .collect()
}

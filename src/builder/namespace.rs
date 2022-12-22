
use std::collections::HashMap;

use clang::{Entity, EntityKind};

use super::{builder::{Builder, AnEntry, get_fully_qualified_name}, class::Class};

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
    pub entries: HashMap<String, Entry<'e>>,
}

impl<'e> AnEntry<'e> for Namespace<'e> {
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

    fn url(&self) -> String {
        String::from("./") + &get_fully_qualified_name(&self.entity).join("/")
    }
}

impl<'e> Namespace<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        let mut ret = Self {
            entity,
            entries: HashMap::new(),
        };
        ret.load_entries();
        ret
    }

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

use std::sync::Arc;

use crate::url::UrlPath;
use clang::Entity;

use super::{
    builder::{
        get_fully_qualified_name, ASTEntry, BuildResult, Builder, Entry, NavItem, OutputEntry,
    },
    shared::output_classlike,
};

pub struct Class<'e> {
    entity: Entity<'e>,
}

impl<'e> Entry<'e> for Class<'e> {
    fn name(&self) -> String {
        self.entity
            .get_display_name()
            .unwrap_or("`Anonymous class`".into())
    }

    fn url(&self) -> UrlPath {
        UrlPath::new_with_path(get_fully_qualified_name(&self.entity))
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), Some(("box", false)))
    }
}

impl<'e> ASTEntry<'e> for Class<'e> {
    fn entity(&self) -> &Entity<'e> {
        &self.entity
    }
}

impl<'e> OutputEntry<'e> for Class<'e> {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, String)>) {
        (
            builder.config.templates.class.clone(),
            output_classlike(self, builder),
        )
    }
}

impl<'e> Class<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self { entity }
    }
}

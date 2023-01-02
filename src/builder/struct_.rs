use std::sync::Arc;

use crate::{html::html::Html, url::UrlPath};
use clang::Entity;

use super::{
    builder::{ASTEntry, BuildResult, Builder, EntityMethods, Entry, NavItem, OutputEntry},
    shared::output_classlike,
};

pub struct Struct<'e> {
    entity: Entity<'e>,
}

impl<'e> Entry<'e> for Struct<'e> {
    fn name(&self) -> String {
        self.entity
            .get_display_name()
            .unwrap_or("`Anonymous struct`".into())
    }

    fn url(&self) -> UrlPath {
        self.entity.rel_url()
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), Some(("box", true)))
    }
}

impl<'e> ASTEntry<'e> for Struct<'e> {
    fn entity(&self) -> &Entity<'e> {
        &self.entity
    }
}

impl<'e> OutputEntry<'e> for Struct<'e> {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        (
            builder.config.templates.struct_.clone(),
            output_classlike(self, builder),
        )
    }
}

impl<'e> Struct<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self { entity }
    }
}

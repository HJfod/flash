use std::sync::Arc;

use crate::{html::html::Html, url::UrlPath};
use clang::Entity;

use super::{
    builder::{ASTEntry, BuildResult, Builder, EntityMethods, Entry, NavItem, OutputEntry},
    shared::output_entity,
};

pub struct Function<'e> {
    entity: Entity<'e>,
}

impl<'e> Entry<'e> for Function<'e> {
    fn name(&self) -> String {
        self.entity
            .get_name()
            .unwrap_or("`Anonymous function`".into())
    }

    fn url(&self) -> UrlPath {
        UrlPath::parse("functions")
            .unwrap()
            .join(self.entity.rel_url())
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), Some(("code", true)))
    }
}

impl<'e> ASTEntry<'e> for Function<'e> {
    fn entity(&self) -> &Entity<'e> {
        &self.entity
    }
}

impl<'e> OutputEntry<'e> for Function<'e> {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        (
            builder.config.templates.function.clone(),
            output_entity(self, builder),
        )
    }
}

impl<'e> Function<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self { entity }
    }
}

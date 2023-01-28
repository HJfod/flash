use std::sync::Arc;

use crate::{html::Html, url::UrlPath};
use clang::Entity;

use super::{
    builder::{ASTEntry, BuildResult, Builder, EntityMethods, Entry, NavItem, OutputEntry},
    shared::output_entity,
};

pub struct Function<'e> {
    entity: Entity<'e>,
}

impl<'e> Function<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self { entity }
    }
}

impl<'e> Entry<'e> for Function<'e> {
    fn name(&self) -> String {
        self.entity
            .get_name()
            .unwrap_or("`Anonymous function`".into())
    }

    fn url(&self) -> UrlPath {
        self.entity.rel_docs_url().expect("Unable to get function URL")
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

    fn category(&self) -> &'static str {
        "function"
    }
}

impl<'e> OutputEntry<'e> for Function<'e> {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        (
            builder.config.templates.function.clone(),
            output_entity(self, builder),
        )
    }

    fn title(&self, builder: &'e Builder<'e>) -> String {
        self.output_title(builder)
    }

    fn description(&self, builder: &'e Builder<'e>) -> String {
        self.output_description(builder)
    }
}

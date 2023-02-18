use std::sync::Arc;

use crate::{html::Html, url::UrlPath};
use clang::Entity;

use super::{
    builder::Builder,
    traits::{ASTEntry, BuildResult, EntityMethods, Entry, NavItem, OutputEntry, SubItem},
    shared::output_classlike,
};

pub struct Class<'e> {
    entity: Entity<'e>,
}

impl<'e> Class<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self { entity }
    }
}

impl<'e> Entry<'e> for Class<'e> {
    fn name(&self) -> String {
        self.entity
            .get_display_name()
            .unwrap_or("`Anonymous class`".into())
    }

    fn url(&self) -> UrlPath {
        self.entity.rel_docs_url().expect("Unable to get class URL")
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(
            &self.name(), self.url(), Some(("box", false)),
            SubItem::for_classlike(&self.entity)
        )
    }
}

impl<'e> ASTEntry<'e> for Class<'e> {
    fn entity(&self) -> &Entity<'e> {
        &self.entity
    }

    fn category(&self) -> &'static str {
        "class"
    }
}

impl<'e> OutputEntry<'e> for Class<'e> {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        (
            builder.config.templates.class.clone(),
            output_classlike(self, builder),
        )
    }

    fn description(&self, builder: &'e Builder<'e>) -> String {
        self.output_description(builder)
    }
}

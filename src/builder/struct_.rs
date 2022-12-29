use std::sync::Arc;

use crate::url::UrlPath;
use clang::Entity;

use super::{
    builder::{AnEntry, Builder, NavItem, OutputEntry, EntityMethods, BuildResult},
    class::output_classlike,
};

pub struct Struct<'e> {
    entity: Entity<'e>,
}

impl<'e> AnEntry<'e> for Struct<'e> {
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

impl<'e> OutputEntry<'e> for Struct<'e> {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, String)>) {
        (
            builder.config.templates.struct_.clone(),
            output_classlike(self, &self.entity, builder),
        )
    }
}

impl<'e> Struct<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self { entity }
    }
}

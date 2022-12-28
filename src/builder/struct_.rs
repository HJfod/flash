use crate::url::UrlPath;
use clang::Entity;

use super::{
    builder::{get_fully_qualified_name, AnEntry, Builder, NavItem, OutputEntry},
    class::output_classlike,
};

pub struct Struct<'e> {
    entity: Entity<'e>,
}

impl<'e> AnEntry<'e> for Struct<'e> {
    fn name(&self) -> String {
        self.entity
            .get_name()
            .unwrap_or("`Anonymous struct`".into())
    }

    fn url(&self) -> UrlPath {
        UrlPath::new_with_path(get_fully_qualified_name(&self.entity))
    }

    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), Some(("box", true)))
    }
}

impl<'c, 'e> OutputEntry<'c, 'e> for Struct<'e> {
    fn output(&self, builder: &Builder<'c, 'e>) -> (&'c String, Vec<(&str, String)>) {
        (
            &builder.config.templates.struct_,
            output_classlike(&self.entity, builder),
        )
    }
}

impl<'e> Struct<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self { entity }
    }
}

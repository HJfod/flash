use std::sync::Arc;

use crate::url::UrlPath;
use clang::Entity;

use super::{
    builder::{AnEntry, Builder, NavItem, OutputEntry, EntityMethods, BuildResult, get_github_url, get_header_path},
};

pub struct Function<'e> {
    entity: Entity<'e>,
}

impl<'e> AnEntry<'e> for Function<'e> {
    fn name(&self) -> String {
        self.entity
            .get_name()
            .unwrap_or("`Anonymous function`".into())
    }

    fn url(&self) -> UrlPath {
        UrlPath::parse("functions").unwrap().join(self.entity.rel_url())
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), Some(("code", true)))
    }
}

impl<'e> OutputEntry<'e> for Function<'e> {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, String)>) {
        (
            builder.config.templates.function.clone(),
            vec![
                // todo: extract these to some entity output thing
                ("name", self.name()),
                (
                    "description",
                    self.entity
                        .get_parsed_comment()
                        .map(|c| c.as_html())
                        .unwrap_or("<p>No Description Provided</p>".into()),
                ),
                (
                    "header_url",
                    get_github_url(builder.config.clone(), &self.entity).unwrap_or(String::new()),
                ),
                (
                    "header_path",
                    get_header_path(builder.config.clone(), &self.entity)
                        .unwrap_or(UrlPath::new())
                        .to_raw_string(),
                ),
            ]
        )
    }
}

impl<'e> Function<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self { entity }
    }
}

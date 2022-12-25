
use clang::Entity;
use crate::url::UrlPath;

use super::builder::{AnEntry, Builder, get_github_url, get_fully_qualified_name, OutputEntry, NavItem, get_header_path};

pub struct Class<'e> {
    entity: Entity<'e>,
}

impl<'e> AnEntry<'e> for Class<'e> {
    fn name(&self) -> String {
        self.entity.get_name().unwrap_or("<Anonymous class or struct>".into())
    }

    fn url(&self) -> UrlPath {
        UrlPath::new_with_path(get_fully_qualified_name(&self.entity))
    }

    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), Some("box"))
    }
}

impl<'c, 'e> OutputEntry<'c, 'e> for Class<'e> {
    fn output(&self, builder: &Builder<'c, 'e>) -> (&'c String, Vec<(String, String)>) {
        (
            &builder.config.templates.class,
            vec![
                ("name".to_string(), self.entity.get_name().unwrap()),
                (
                    "description".into(),
                    self.entity
                        .get_parsed_comment()
                        .map(|c| c.as_html())
                        .unwrap_or("<p>No Description Provided</p>".into()),
                ),
                (
                    "header_url".into(),
                    get_github_url(builder.config, &self.entity).unwrap_or(String::new()),
                ),
                (
                    "header_path".into(),
                    get_header_path(builder.config, &self.entity).unwrap_or(UrlPath::new()).to_raw_string(),
                ),
            ]
        )
    }
}

impl<'e> Class<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self {
            entity
        }
    }
}

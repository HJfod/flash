
use clang::Entity;
use super::builder::{AnEntry, Builder, get_header_url, get_fully_qualified_name, OutputEntry, NavItem};

pub struct Class<'e> {
    entity: Entity<'e>,
}

impl<'e> AnEntry<'e> for Class<'e> {
    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(
            &self.name(),
            &get_fully_qualified_name(&self.entity).join("/"),
            Some("box")
        )
    }

    fn name(&self) -> String {
        self.entity.get_name().unwrap_or("<Anonymous class or struct>".into())
    }

    fn url(&self) -> String {
        String::from("./") + &get_fully_qualified_name(&self.entity).join("/")
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
                    "header_link".into(),
                    get_header_url(builder.config, &self.entity)
                        .map(|url| format!("<a href='{}'>View Header</a>", url))
                        .unwrap_or(String::new()),
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

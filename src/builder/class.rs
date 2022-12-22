
use std::fs;
use clang::Entity;
use super::builder::{AnEntry, Builder, write_docs_output, get_header_url, get_fully_qualified_name};

pub struct Class<'e> {
    entity: Entity<'e>,
}

impl<'e> AnEntry<'e> for Class<'e> {
    fn build(&self, builder: &Builder<'_, 'e>) -> Result<(), String> {
        // Target directory
        let dir_path = builder.config.output_dir.join(&self.url());
        fs::create_dir_all(&dir_path).unwrap();
    
        write_docs_output(
            builder,
            &builder.config.presentation.class_template,
            &self.url(),
            [
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
        )?;
    
        Ok(())
    }

    fn build_nav(&self, relative: &String) -> String {
        format!(
            "<a href='.{}/{}'>
                <i data-feather='box' class='class-icon'></i>
                {}
            </a>",
            "/..".repeat(relative.matches("/").count()),
            get_fully_qualified_name(&self.entity).join("/"),
            self.name()
        )
    }

    fn name(&self) -> String {
        self.entity.get_name().unwrap_or("<Anonymous class or struct>".into())
    }

    fn url(&self) -> String {
        String::from("./") + &get_fully_qualified_name(&self.entity).join("/")
    }
}

impl<'e> Class<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self {
            entity
        }
    }
}

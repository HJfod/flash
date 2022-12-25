
use crate::url::UrlPath;

use super::builder::{AnEntry, OutputEntry, Builder, NavItem};

pub struct Index {}

impl<'e> AnEntry<'e> for Index {
    fn name(&self) -> String {
        "Home".into()
    }

    fn url(&self) -> UrlPath {
        UrlPath::new()
    }

    fn build(&self, builder: &super::builder::Builder<'_, 'e>) -> Result<(), String> {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), None)
    }
}

impl<'c, 'e> OutputEntry<'c, 'e> for Index {
    fn output(&self, builder: &Builder<'c, 'e>) -> (&'c String, Vec<(String, String)>) {
        (
            &builder.config.templates.index,
            Vec::new(),
        )
    }
}

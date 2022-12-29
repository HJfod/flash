use std::sync::Arc;
use crate::url::UrlPath;
use super::builder::{AnEntry, Builder, NavItem, OutputEntry, BuildResult};

pub struct Index {}

impl<'e> AnEntry<'e> for Index {
    fn name(&self) -> String {
        "Home".into()
    }

    fn url(&self) -> UrlPath {
        UrlPath::new()
    }

    fn build(&self, builder: &super::builder::Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), None)
    }
}

impl<'e> OutputEntry<'e> for Index {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, String)>) {
        (builder.config.templates.index.clone(), Vec::new())
    }
}

use super::builder::{BuildResult, Builder, Entry, NavItem, OutputEntry};
use crate::{html::html::Html, url::UrlPath};
use std::sync::Arc;

pub struct Index {}

impl<'e> Entry<'e> for Index {
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
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, Html)>) {
        (builder.config.templates.index.clone(), Vec::new())
    }
}

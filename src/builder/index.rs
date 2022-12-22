
use super::builder::{AnEntry, OutputEntry, Builder};

pub struct Index {}

impl<'e> AnEntry<'e> for Index {
    fn name(&self) -> String {
        "[index]".into()
    }

    fn url(&self) -> String {
        ".".into()
    }

    fn build(&self, builder: &super::builder::Builder<'_, 'e>) -> Result<(), String> {
        builder.create_output_for(self)
    }

    fn build_nav(&self, _: &String) -> String {
        String::new()
    }
}

impl<'c, 'e> OutputEntry<'c, 'e> for Index {
    fn output(&self, builder: &Builder<'c, 'e>) -> (&'c String, Vec<(String, String)>) {
        (
            &builder.config.presentation.index_template,
            Vec::new(),
        )
    }
}

use std::sync::Arc;

use crate::url::UrlPath;
use clang::{Accessibility, Entity, EntityKind};

use super::{
    builder::{
        get_fully_qualified_name, get_github_url, get_header_path, AnEntry, Builder, NavItem,
        OutputEntry, sanitize_html, BuildResult,
    },
    links::{fmt_field, fmt_fun_decl, fmt_section},
};

pub struct Class<'e> {
    entity: Entity<'e>,
}

impl<'e> AnEntry<'e> for Class<'e> {
    fn name(&self) -> String {
        self.entity.get_display_name().unwrap_or("`Anonymous class`".into())
    }

    fn url(&self) -> UrlPath {
        UrlPath::new_with_path(get_fully_qualified_name(&self.entity))
    }

    fn build(&self, builder: &Builder<'e>) -> BuildResult {
        builder.create_output_for(self)
    }

    fn nav(&self) -> NavItem {
        NavItem::new_link(&self.name(), self.url(), Some(("box", false)))
    }
}

impl<'e> OutputEntry<'e> for Class<'e> {
    fn output(&self, builder: &Builder<'e>) -> (Arc<String>, Vec<(&'static str, String)>) {
        (
            builder.config.templates.class.clone(),
            output_classlike(self, &self.entity, builder),
        )
    }
}

pub fn output_classlike<'e, T: AnEntry<'e>>(entry: &T, entity: &Entity<'e>, builder: &Builder) -> Vec<(&'static str, String)> {
    vec![
        ("name", sanitize_html(&entry.name())),
        (
            "description",
            entity
                .get_parsed_comment()
                .map(|c| c.as_html())
                .unwrap_or("<p>No Description Provided</p>".into()),
        ),
        (
            "header_url",
            get_github_url(builder.config.clone(), &entity).unwrap_or(String::new()),
        ),
        (
            "header_path",
            get_header_path(builder.config.clone(), &entity)
                .unwrap_or(UrlPath::new())
                .to_raw_string(),
        ),
        (
            "public_static_functions",
            fmt_section(
                "Public static methods",
                entity
                    .get_children()
                    .iter()
                    .filter(|child| {
                        child.get_kind() == EntityKind::Method
                            && child.is_static_method()
                            && child.get_accessibility() == Some(Accessibility::Public)
                    })
                    .map(|e| fmt_fun_decl(e, builder.config.clone()))
                    .collect::<Vec<_>>(),
            ),
        ),
        (
            "public_member_functions",
            fmt_section(
                "Public member functions",
                entity
                    .get_children()
                    .iter()
                    .filter(|child| {
                        child.get_kind() == EntityKind::Method
                            && !child.is_static_method()
                            && child.get_accessibility() == Some(Accessibility::Public)
                    })
                    .map(|e| fmt_fun_decl(e, builder.config.clone()))
                    .collect::<Vec<_>>(),
            ),
        ),
        (
            // todo: hide if final class
            "protected_member_functions",
            fmt_section(
                "Protected member functions",
                entity
                    .get_children()
                    .iter()
                    .filter(|child| {
                        child.get_kind() == EntityKind::Method
                            && !child.is_static_method()
                            && child.get_accessibility() == Some(Accessibility::Protected)
                    })
                    .map(|e| fmt_fun_decl(e, builder.config.clone()))
                    .collect::<Vec<_>>(),
            ),
        ),
        (
            "public_members",
            fmt_section(
                "Fields",
                entity
                    .get_children()
                    .iter()
                    .filter(|child| {
                        child.get_kind() == EntityKind::FieldDecl
                            && child.get_accessibility() == Some(Accessibility::Public)
                    })
                    .map(|e| fmt_field(e, builder.config.clone()))
                    .collect::<Vec<_>>(),
            ),
        ),
    ]
}

impl<'e> Class<'e> {
    pub fn new(entity: Entity<'e>) -> Self {
        Self { entity }
    }
}

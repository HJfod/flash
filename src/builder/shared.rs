use clang::{Accessibility, EntityKind, Entity, Type, TypeKind};
use crate::url::UrlPath;
use super::{
    builder::{get_github_url, get_header_path, sanitize_html, ASTEntry, Builder},
};
use std::sync::Arc;
use super::builder::{get_ancestorage, EntityMethods};
use crate::config::Config;

fn fmt_type(entity: &Type, config: Arc<Config>) -> String {
    let base = entity.get_pointee_type().unwrap_or(entity.to_owned());
    let decl = base.get_declaration();
    let link = decl.map(|decl| decl.docs_url(config.clone()));
    let kind = decl
        .map(|decl| decl.get_kind())
        .unwrap_or(EntityKind::UnexposedDecl);
    let name = decl
        .map(|decl| {
            get_ancestorage(&decl)
                .iter()
                .map(|e| {
                    format!(
                        "<span class='{} name'>{}</span>",
                        match e.get_kind() {
                            EntityKind::Namespace => "namespace",
                            EntityKind::ClassDecl => "class",
                            EntityKind::ClassTemplate => "class",
                            EntityKind::StructDecl => "struct",
                            EntityKind::FunctionDecl => "fun",
                            EntityKind::TypedefDecl => "alias",
                            EntityKind::UsingDeclaration => "alias",
                            EntityKind::TypeAliasDecl => "alias",
                            EntityKind::EnumDecl => "enum",
                            _ => "type",
                        },
                        e.get_name().unwrap_or("_".into())
                    )
                })
                .collect::<Vec<_>>()
                .join("<span class='scope'>::</span>")
        })
        .unwrap_or_else(|| {
            format!(
                "<span class='{} name'>{}</span>",
                if base.is_pod() {
                    "keyword"
                } else {
                    "template-param"
                },
                match base.get_kind() {
                    TypeKind::Void => "void".into(),
                    TypeKind::Bool => "bool".into(),
                    TypeKind::Long => "long".into(),
                    TypeKind::Auto => "auto".into(),
                    TypeKind::Int => "int".into(),
                    TypeKind::Short => "short".into(),
                    TypeKind::SChar | TypeKind::CharS => "char".into(),
                    TypeKind::UChar | TypeKind::CharU => "uchar".into(),
                    TypeKind::Float => "float".into(),
                    TypeKind::Double => "double".into(),
                    TypeKind::UInt => "uint".into(),
                    TypeKind::LongLong => "long long".into(),
                    _ => base.get_display_name(),
                }
            )
        });

    format!(
        "<a class='entity type {css}' {link}>
            {name}{template}{const}{ref}
        </a>",
        css = format!(
            "{} {}",
            if entity.is_pod() { "keyword" } else { "" },
            if link.is_none() { "disabled" } else { "" },
        ),

        link = link
            .map(|link| format!("href='{link}' onclick='return navigate(\"{link}\")'"))
            .unwrap_or(String::new()),

        template = match kind {
            EntityKind::TypeAliasDecl | EntityKind::TypedefDecl => String::new(),
            _ => base.get_template_argument_types().map(|types| {
                format!(
                    "&lt;{}&gt;",
                    types
                        .iter()
                        .map(|t| t.map(|t| fmt_type(&t, config.clone())).unwrap_or(String::from("_unk")))
                        .collect::<Vec<_>>()
                        .join("<span class='comma space-after'>,</span>")
                )
            }).unwrap_or(String::new()),
        },

        const = if base.is_const_qualified() {
            "<span class='keyword space-before'>const</span>"
        } else {
            ""
        },

        ref = match entity.get_kind() {
            TypeKind::LValueReference => "&",
            TypeKind::RValueReference => "&&",
            TypeKind::Pointer => "*",
            _ => "",
        },
    )
}

fn fmt_param(param: &Entity, config: Arc<Config>) -> String {
    format!(
        "<div class='entity var'>{}{}</div>",
        param
            .get_type()
            .map(|t| fmt_type(&t, config))
            .unwrap_or(String::new()),
        param
            .get_display_name()
            .map(|n| format!("<span class='name space-before'>{}</span>", n))
            .unwrap_or(String::new())
    )
}

pub fn fmt_field(field: &Entity, config: Arc<Config>) -> String {
    format!(
        "<div class='entity var'>{};</div>",
        fmt_param(field, config)
    )
}

pub fn fmt_fun_decl(fun: &Entity, config: Arc<Config>) -> String {
    format!(
        "<details class='entity-desc'>
            <summary class='entity fun'>
                {static}{virtual}{return}
                <span class='name space-before'>{name}</span>
                <span class='params'>({params})</span>{const}{pure};
            </summary>
            <div>
                {description}
            </div>
        </details>",

        static = if fun.is_static_method() {
            "<span class='keyword space-after'>static</span>"
        } else { "" },

        virtual = if fun.is_virtual_method() {
            "<span class='keyword space-after'>virtual</span>"
        } else { "" },

        return = fun.get_result_type().map(|t| fmt_type(&t, config.clone())).unwrap_or(String::new()),

        name = fun.get_name().unwrap_or(String::from("_anon")),

        params = fun.get_arguments().map(|args|
            args
                .iter()
                .map(|arg| fmt_param(arg, config.clone()))
                .collect::<Vec<_>>()
                .join("<span class='comma space-after'>,</span>")
        ).unwrap_or(String::new()),

        const = if fun.is_const_method() {
            "<span class='keyword space-before'>const</span>"
        } else { "" },

        pure = if fun.is_pure_virtual_method() {
            "<span class='space-before'>=</span>
            <span class='literal space-before'>0</span>"
        } else { "" },

        description = fun.get_parsed_comment().map(|p| p.as_html()).unwrap_or(String::from("<p>No description provided.</p>"))
    )
}

pub fn fmt_section(title: &str, data: Vec<String>) -> String {
    format!(
        "<details open class='section'>
            <summary>
                <span>
                    <i data-feather='chevron-right'></i>
                    {title}
                    <span class='badge'>{}</span>
                </span>
            </summary>
            <div>
                {}
            </div>
        </details>",
        data.len(),
        data.join("\n")
    )
}

pub fn output_entity<'e, T: ASTEntry<'e>>(
    entry: &T,
    builder: &Builder,
) -> Vec<(&'static str, String)> {
    vec![
        ("name", sanitize_html(&entry.name())),
        (
            "description",
            entry
                .entity()
                .get_parsed_comment()
                .map(|c| c.as_html())
                .unwrap_or("<p>No Description Provided</p>".into()),
        ),
        (
            "header_url",
            get_github_url(builder.config.clone(), entry.entity()).unwrap_or(String::new()),
        ),
        (
            "header_path",
            get_header_path(builder.config.clone(), entry.entity())
                .unwrap_or(UrlPath::new())
                .to_raw_string(),
        ),
    ]
}

pub fn output_classlike<'e, T: ASTEntry<'e>>(
    entry: &T,
    builder: &Builder,
) -> Vec<(&'static str, String)> {
    let mut ent = output_entity(entry, builder);
    ent.extend(vec![
        (
            "public_static_functions",
            fmt_section(
                "Public static methods",
                entry
                    .entity()
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
                entry
                    .entity()
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
                entry
                    .entity()
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
                entry
                    .entity()
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
    ]);
    ent
}

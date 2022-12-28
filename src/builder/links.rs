
use clang::{Entity, EntityKind, Type, TypeKind};
use crate::{url::UrlPath, config::{Config}};
use super::builder::{get_ancestorage, get_fully_qualified_name};

fn fmt_type(entity: &Type, config: &Config) -> String {
    let base = entity.get_pointee_type().unwrap_or(entity.to_owned());
    let link = base.get_declaration().map(|decl| {
        UrlPath::new_with_path(get_fully_qualified_name(&decl)).to_absolute(config)
    });
    let name = base.get_declaration().map(|decl| {
        get_ancestorage(&decl)
            .iter()
            .map(|e| format!(
                "<span class='{} name'>{}</span>",
                match e.get_kind() {
                    EntityKind::Namespace => "namespace",
                    EntityKind::ClassDecl => "class",
                    EntityKind::StructDecl => "struct",
                    EntityKind::FunctionDecl => "fun",
                    EntityKind::TypedefDecl => "alias",
                    EntityKind::UsingDeclaration => "alias",
                    EntityKind::TypeAliasDecl => "alias",
                    EntityKind::EnumDecl => "enum",
                    _ => "type",
                },
                e.get_name().unwrap_or("_anon".into())
            ))
            .collect::<Vec<_>>()
            .join("<span class='scope'>::</span>")
    }).unwrap_or_else(||
        format!(
            "<span class='keyword name'>{}</span>",
            match base.get_kind() {
                TypeKind::Void     => "void",
                TypeKind::Bool     => "bool",
                TypeKind::Long     => "long",
                TypeKind::Auto     => "auto",
                TypeKind::Int      => "int",
                TypeKind::Short    => "short",
                TypeKind::SChar | TypeKind::CharS => "char",
                TypeKind::UChar | TypeKind::CharU => "uchar",
                TypeKind::Float    => "float",
                TypeKind::Double   => "double",
                TypeKind::UInt     => "uint",
                TypeKind::LongLong => "long long",
                _                  => "_unk_builtin",
            }
        )
    );

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

        template = base.get_template_argument_types().map(|types| {
            format!(
                "&lt;{}&gt;",
                types
                    .iter()
                    .map(|t| t.map(|t| fmt_type(&t, config)).unwrap_or(String::from("_unk")))
                    .collect::<Vec<_>>()
                    .join("<span class='comma space-after'>,</span>")
            )
        }).unwrap_or(String::new()),

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

fn fmt_param(param: &Entity, config: &Config) -> String {
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

pub fn fmt_field(field: &Entity, config: &Config) -> String {
    format!("<div class='entity var'>{};</div>", fmt_param(field, config))
}

pub fn fmt_fun_decl(fun: &Entity, config: &Config) -> String {
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

        return = fun.get_result_type().map(|t| fmt_type(&t, config)).unwrap_or(String::new()),

        name = fun.get_name().unwrap_or(String::from("_anon")),

        params = fun.get_arguments().map(|args|
            args
                .iter()
                .map(|arg| fmt_param(arg, config))
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

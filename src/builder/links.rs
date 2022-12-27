
use clang::{Entity, Type, EntityKind, TypeKind};

use super::builder::{get_ancestorage};

pub fn fmt_type(entity: &Type) -> String {
    let Some(decl) = entity
        .get_pointee_type()
        .unwrap_or(entity.to_owned())
        .get_declaration()
    else {
        return format!(
            "<text class='entity type pod'>{}</text>",
            entity.get_display_name()
        );
    };

    format!(
        "<a class='entity type {}' href='#'>{}{}{}</a>",
        if entity.is_pod() { "keyword" } else { "" },

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
                    _ => "type",
                },
                e.get_name().unwrap_or("_anon".into())
            ))
            .collect::<Vec<_>>()
            .join("<span class='scope'>::</span>"),

        if entity.is_const_qualified() {
            "<span class='keyword'>const</span>"
        } else { "" },

        match entity.get_kind() {
            TypeKind::LValueReference => "&",
            TypeKind::RValueReference => "&&",
            TypeKind::Pointer => "*",
            _ => "",
        },
    )
}

pub fn fmt_param(param: &Entity) -> String {
    format!(
        "<div class='entity var'>{}{}</div>",
        param.get_type().map(|t| fmt_type(&t)).unwrap_or(String::new()),
        param.get_display_name()
            .map(|n| format!("<span class='name'>{}</span>", n))
            .unwrap_or(String::new())
    )
}

pub fn fmt_fun_decl(fun: &Entity) -> String {
    format!(
        "<div class='entity fun'>{}<span class='name'>{}</span>({})</div>",
        fun.get_result_type().map(|t| fmt_type(&t)).unwrap_or(String::new()),
        fun.get_name().unwrap_or(String::from("_anon")),
        fun.get_arguments().map(|args|
            args.iter().map(|arg| fmt_param(arg)).collect::<Vec<_>>().join(", ")
        ).unwrap_or(String::new())
    )
}

use super::builder::EntityMethods;
use super::builder::{ASTEntry, Builder};
use crate::config::Config;
use crate::{
    html::html::{Html, HtmlElement, HtmlList, HtmlText},
    url::UrlPath,
};
use clang::{
    documentation::{Comment, CommentChild, InlineCommandStyle},
    Accessibility, Entity, EntityKind, Type, TypeKind,
};
use std::sync::Arc;

trait Surround<T> {
    fn surround(self, start: T, end: T) -> Self;
}

impl<T> Surround<T> for Vec<T> {
    fn surround(mut self, start: T, end: T) -> Self {
        self.insert(0, start);
        self.push(end);
        self
    }
}

trait InsertBetween<T, Sep: Fn() -> T> {
    fn insert_between(self, separator: Sep) -> Self;
}

impl<T, Sep: Fn() -> T> InsertBetween<T, Sep> for Vec<T> {
    fn insert_between(self, separator: Sep) -> Self {
        let mut res = Vec::new();
        let mut first = true;
        for item in self.into_iter() {
            if !first {
                res.push(separator());
            }
            first = false;
            res.push(item);
        }
        res
    }
}

fn fmt_comment_children(parent: HtmlElement, children: Vec<CommentChild>) -> Html {
    let mut stack = vec![parent];

    // Collect all parameter commands to one parent section
    let mut params = HtmlElement::new("section").with_class("params");
    let mut template_params = HtmlElement::new("section").with_classes(&["params", "template"]);

    for child in children {
        match child {
            CommentChild::Text(text) => {
                stack.last_mut().unwrap().add_child(HtmlText::new(text));
            }

            CommentChild::Paragraph(children) => {
                stack
                    .last_mut()
                    .unwrap()
                    .add_child(fmt_comment_children(HtmlElement::new("p"), children));
            }

            CommentChild::HtmlStartTag(tag) => {
                // Add self-closing tags to the top-most stack member as normal
                if tag.closing {
                    stack
                        .last_mut()
                        .unwrap()
                        .add_child(HtmlElement::new(&tag.name).with_attrs(&tag.attributes));
                }
                // Otherwise add tag as the topmost member in stack
                else {
                    stack.push(HtmlElement::new(&tag.name).with_attrs(&tag.attributes));
                }
            }

            CommentChild::HtmlEndTag(_) => {
                // Pop the topmost member in stack
                // We assume that all doc comments are valid HTML; as in, all
                // HTML tags have a valid closing tag, and that there are no
                // closing tags without a matching opening tag.
                let pop = stack.pop().unwrap();
                stack.last_mut().unwrap().add_child(pop);
            }

            CommentChild::ParamCommand(cmd) => {
                params.add_child(Html::p(cmd.parameter));
                params.add_child(fmt_comment_children(
                    HtmlElement::new("div").with_class("description"),
                    cmd.children,
                ));
            }

            CommentChild::TParamCommand(cmd) => {
                template_params.add_child(Html::p(cmd.parameter));
                template_params.add_child(fmt_comment_children(
                    HtmlElement::new("div").with_class("description"),
                    cmd.children,
                ));
            }

            CommentChild::BlockCommand(cmd) => match cmd.command.as_str() {
                "return" | "returns" => stack.last_mut().unwrap().add_child(
                    HtmlElement::new("section")
                        .with_child(Html::p("Returns"))
                        .with_child(fmt_comment_children(
                            HtmlElement::new("div").with_class("description"),
                            cmd.children,
                        )),
                ),

                _ => println!("Warning: Unknown command {}", cmd.command),
            },

            CommentChild::InlineCommand(cmd) => match cmd.command.as_str() {
                "return" | "returns" => stack.last_mut().unwrap().add_child(
                    HtmlElement::new("section")
                        .with_class_opt(cmd.style.map(|style| match style {
                            InlineCommandStyle::Bold => "bold",
                            InlineCommandStyle::Emphasized => "em",
                            InlineCommandStyle::Monospace => "mono",
                        }))
                        .with_child(Html::p("Returns"))
                        .with_child(Html::p(cmd.arguments.join(" "))),
                ),

                _ => println!("Warning: Unknown command {}", cmd.command),
            },

            _ => {
                println!("Unsupported comment option {:?}", child);
            }
        }
    }

    // Get first element (parent) back from stack
    let mut res = stack.into_iter().nth(0).unwrap();

    // Add params if this comment had any
    if params.has_children() {
        res.add_child(params);
    }
    if template_params.has_children() {
        res.add_child(template_params);
    }

    res.into()
}

fn fmt_comment(comment: Comment) -> Html {
    fmt_comment_children(
        HtmlElement::new("div").with_class("description"),
        comment.get_children(),
    )
}

fn fmt_type(entity: &Type, config: Arc<Config>) -> Html {
    let base = entity.get_pointee_type().unwrap_or(entity.to_owned());
    let decl = base.get_declaration();
    let link = decl.map(|decl| decl.docs_url(config.clone()));
    let kind = decl
        .map(|decl| decl.get_kind())
        .unwrap_or(EntityKind::UnexposedDecl);

    let name: Html = decl
        .map(|decl| {
            HtmlList::new(
                decl.get_ancestorage()
                    .iter()
                    .map(|e| {
                        HtmlElement::new("span")
                            .with_class(match e.get_kind() {
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
                            })
                            .with_class("name")
                            .with_child(HtmlText::new(e.get_name().unwrap_or("_".into())))
                            .into()
                    })
                    .collect::<Vec<_>>()
                    .insert_between(|| Html::span(&["scope"], "::")),
            )
            .into()
        })
        .unwrap_or_else(|| {
            HtmlElement::new("span")
                .with_class(if base.is_pod() {
                    "keyword"
                } else {
                    "template-param"
                })
                .with_class("name")
                .with_child(HtmlText::new(match base.get_kind() {
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
                }))
                .into()
        });

    HtmlElement::new("a")
        .with_class("entity")
        .with_class("type")
        .with_class_opt(entity.is_pod().then_some("keyword"))
        .with_class_opt(link.is_none().then_some("disabled"))
        .with_attr_opt("href", link.clone())
        .with_attr_opt(
            "onclick",
            link.map(|link| format!("return navigate('{link}'")),
        )
        .with_child(name)
        .with_child_opt(match kind {
            EntityKind::TypeAliasDecl | EntityKind::TypedefDecl => None,
            _ => base.get_template_argument_types().map(|types| {
                HtmlList::new(
                    types
                        .iter()
                        .map(|t| {
                            t.map(|t| fmt_type(&t, config.clone()))
                                .unwrap_or(HtmlText::new("_unk").into())
                        })
                        .collect::<Vec<_>>()
                        .insert_between(|| {
                            HtmlElement::new("span")
                                .with_class("comma")
                                .with_class("space-after")
                                .with_child(HtmlText::new(","))
                                .into()
                        })
                        .surround(HtmlText::new("<").into(), HtmlText::new(">").into()),
                )
            }),
        })
        .with_child_opt(
            base.is_const_qualified()
                .then_some(Html::span(&["keyword", "space-before"], "const")),
        )
        .with_child_opt(match entity.get_kind() {
            TypeKind::LValueReference => Some::<Html>(HtmlText::new("&").into()),
            TypeKind::RValueReference => Some(HtmlText::new("&&").into()),
            TypeKind::Pointer => Some(HtmlText::new("*").into()),
            _ => None,
        })
        .into()
}

fn fmt_param(param: &Entity, config: Arc<Config>) -> Html {
    HtmlElement::new("div")
        .with_classes(&["entity", "var"])
        .with_child_opt(param.get_type().map(|t| fmt_type(&t, config)))
        .with_child_opt(
            param
                .get_display_name()
                .map(|name| Html::span(&["name", "space-before"], &name)),
        )
        .into()
}

pub fn fmt_field(field: &Entity, config: Arc<Config>) -> Html {
    HtmlElement::new("div")
        .with_classes(&["entity", "var"])
        .with_child(fmt_param(field, config))
        .with_child(HtmlText::new(";"))
        .into()
}

pub fn fmt_fun_decl(fun: &Entity, config: Arc<Config>) -> Html {
    HtmlElement::new("details")
        .with_class("entity-desc")
        .with_child(
            HtmlElement::new("summary")
                .with_classes(&["entity", "fun"])
                .with_child_opt(
                    fun.is_static_method()
                        .then_some(Html::span(&["keyword", "space-after"], "static")),
                )
                .with_child_opt(
                    fun.is_virtual_method()
                        .then_some(Html::span(&["keyword", "space-after"], "virtual")),
                )
                .with_child_opt(fun.get_result_type().map(|t| fmt_type(&t, config.clone())))
                .with_child(Html::span(
                    &["name", "space-before"],
                    &fun.get_name().unwrap_or("_anon".into()),
                ))
                .with_child(
                    HtmlElement::new("span").with_class("params").with_children(
                        fun.get_arguments()
                            .map(|args| {
                                args.iter()
                                    .map(|arg| fmt_param(arg, config.clone()))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or(Vec::new())
                            .insert_between(|| Html::span(&["comma", "space-after"], ","))
                            .surround(HtmlText::new("(").into(), HtmlText::new(")").into()),
                    ),
                )
                .with_child_opt(
                    fun.is_const_method()
                        .then_some(Html::span(&["keyword", "space-before"], "const")),
                )
                .with_child_opt(
                    fun.is_pure_virtual_method().then_some::<Html>(
                        HtmlList::new(vec![
                            Html::span(&["space-before"], "="),
                            Html::span(&["space-before", "literal"], "0"),
                        ])
                        .into(),
                    ),
                ),
        )
        .with_child(
            HtmlElement::new("div").with_child(
                fun.get_parsed_comment()
                    .map(|p| fmt_comment(p))
                    .unwrap_or(Html::p("No description provided")),
            ),
        )
        .into()
}

pub fn fmt_section(title: &str, data: Vec<Html>) -> Html {
    HtmlElement::new("details")
        .with_attr("open", "")
        .with_class("section")
        .with_child(
            HtmlElement::new("summary").with_child(
                HtmlElement::new("span")
                    .with_child(Html::feather("chevron-right"))
                    .with_child(HtmlText::new(title))
                    .with_child(Html::span(&["badge"], &data.len().to_string())),
            ),
        )
        .with_child(HtmlElement::new("div").with_child(HtmlList::new(data)))
        .into()
}

fn get_github_url(config: Arc<Config>, entity: &Entity) -> Option<String> {
    let path = entity
        .get_definition()?
        .get_location()?
        .get_file_location()
        .file?
        .get_path();

    Some(
        config.project.tree.clone()?
            + &UrlPath::try_from(
                &path
                    .strip_prefix(&config.input_dir)
                    .unwrap_or(&path)
                    .to_path_buf(),
            )
            .ok()?
            .to_string(),
    )
}

fn get_header_path(config: Arc<Config>, entity: &Entity) -> Option<UrlPath> {
    let path = entity
        .get_definition()?
        .get_location()?
        .get_file_location()
        .file?
        .get_path();

    let rel_path = path.strip_prefix(&config.input_dir).unwrap_or(&path);

    for src in &config.sources {
        if rel_path.starts_with(src.dir.to_pathbuf()) {
            if let Some(ref prefix) = src.strip_include_prefix {
                return Some(
                    UrlPath::try_from(&rel_path.strip_prefix(prefix).ok()?.to_path_buf()).ok()?,
                );
            } else {
                return Some(UrlPath::try_from(&rel_path.to_path_buf()).ok()?);
            }
        }
    }

    None
}

pub fn output_entity<'e, T: ASTEntry<'e>>(
    entry: &T,
    builder: &Builder,
) -> Vec<(&'static str, Html)> {
    vec![
        ("name", HtmlText::new(&entry.name()).into()),
        (
            "description",
            entry
                .entity()
                .get_parsed_comment()
                .map(|c| fmt_comment(c))
                .unwrap_or(Html::p("No Description Provided"))
                .into(),
        ),
        (
            "header_url",
            HtmlText::new(
                get_github_url(builder.config.clone(), entry.entity()).unwrap_or(String::new()),
            )
            .into(),
        ),
        (
            "header_path",
            HtmlText::new(
                get_header_path(builder.config.clone(), entry.entity())
                    .unwrap_or(UrlPath::new())
                    .to_raw_string(),
            )
            .into(),
        ),
    ]
}

pub fn output_classlike<'e, T: ASTEntry<'e>>(
    entry: &T,
    builder: &Builder,
) -> Vec<(&'static str, Html)> {
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

use std::{collections::HashMap, fs, str::Chars};

use clang::{
    token::{Token, TokenKind},
    Entity, EntityKind,
};
use multipeek::{IteratorExt, MultiPeek};

use crate::{
    html::{Html, HtmlElement, HtmlList, HtmlText},
    url::UrlPath,
};

use super::{
    builder::{Builder, EntityMethods},
    shared::{fmt_autolinks, fmt_markdown},
};

struct CommentLexer<'s> {
    raw: MultiPeek<Chars<'s>>,
}

impl<'s> CommentLexer<'s> {
    pub fn new(raw: &'s str) -> Self {
        Self {
            raw: raw
                .trim_end_matches("*/")
                .trim_start_matches("/*")
                .chars()
                .multipeek(),
        }
    }

    fn skip_while<P: FnMut(char) -> bool>(&mut self, mut pred: P) -> usize {
        let mut count = 0;
        while self.raw.peek().is_some_and(|c| pred(*c)) {
            self.raw.next();
            count += 1;
        }
        count
    }

    fn skip_to_next_line(&mut self, indentation: Option<usize>) -> usize {
        // Eat all whitespace including newlines
        self.skip_while(|c| c.is_whitespace());

        // Make sure this line was started with a star
        if self.raw.peek().is_some_and(|c| *c == '*') {
            // Consume the star
            self.raw.next();
            // If indentation was provided, remove that amount of whitespace if possible
            if let Some(max) = indentation {
                let mut i = 0;
                self.skip_while(|c| {
                    c.is_whitespace() && c != '\n' && {
                        i += 1;
                        i
                    } <= max
                });
                i
            }
            // Otherwise consume as much whitespace as possible and return the count
            else {
                self.skip_while(|c| c.is_whitespace() && c != '\n')
            }
        }
        // If it wasn't, then cut off all indentation (what we have done)
        else {
            0
        }
    }

    fn skip_to_next_value(&mut self) {
        // Eat whitespace and stars until something that isn't those is found
        self.skip_while(|c| c.is_whitespace() || c == '*');
    }

    fn eat_until<P: FnMut(char) -> bool>(&mut self, mut pred: P) -> Option<String> {
        let mut res = String::new();
        let mut indent_size = None;
        while let Some(c) = self.raw.peek().copied() {
            if pred(c) {
                break;
            }
            // On newlines, skip whitespace and the next line's starting star
            // if there is one, compressing everything to one space
            if c == '\n' {
                let i = self.skip_to_next_line(indent_size);
                if indent_size.is_none() {
                    indent_size = i.into();
                }
                res.push('\n');
            } else {
                self.raw.next();
                res.push(c);
            }
        }
        // println!("indent_size: {:?}", indent_size);
        (!res.is_empty()).then_some(res)
    }

    fn eat_word(&mut self) -> Option<String> {
        self.eat_until(|c| c.is_whitespace())
    }

    pub fn next_command(&mut self) -> Option<ParsedCommand> {
        // Skip whitespace
        self.skip_to_next_value();

        // todo: handle escaped @ symbol

        match self.raw.peek()? {
            '@' => {
                // Consume param symbol
                self.raw.next();
                // Eat command name
                let cmd = self.eat_until(|c| c.is_whitespace() || c == '[')?;
                // Parse attributes if those are provided
                if self.raw.peek().is_some_and(|c| *c == '[') {
                    // Consume opening bracket
                    self.raw.next();
                    let mut attrs = HashMap::new();
                    loop {
                        let Some(key) = self
                            .eat_until(|c| matches!(c, '=' | ']' | ','))
                            .map(|s| s.trim().to_owned())
                        else {
                            break;
                        };

                        // Value provided
                        if self.raw.peek().is_some_and(|c| *c == '=') {
                            // Consume =
                            self.raw.next();

                            // Eat value
                            let value = self
                                .eat_until(|c| matches!(c, ']' | ','))
                                .map(|s| s.trim().to_owned())
                                .unwrap_or(String::new());

                            attrs.insert(key, Some(value));
                        }
                        // No value provided
                        else {
                            attrs.insert(key, None);
                        }

                        // Next value or end of list
                        let Some(next) = self.raw.next() else { break; };
                        match next {
                            ']' => break,
                            ',' => continue,
                            _ => unreachable!(),
                        }
                    }
                    Some(ParsedCommand::new_with(cmd, attrs))
                } else {
                    Some(ParsedCommand::new(cmd))
                }
            }
            _ => Some(ParsedCommand::new("description")),
        }
    }

    pub fn next_param(&mut self) -> Option<String> {
        self.skip_to_next_value();
        self.eat_word()
    }

    pub fn next_value(&mut self) -> Option<String> {
        self.eat_until(|c| c == '@').map(|s| s.trim().to_owned())
    }

    pub fn param_for(&mut self, cmd: &ParsedCommand) -> String {
        self.next_param().unwrap_or_else(|| {
            println!(
                "Warning parsing JSDoc comment: Expected parameter for command {}",
                cmd.cmd
            );
            String::new()
        })
    }

    pub fn value_for(&mut self, cmd: &ParsedCommand) -> String {
        self.next_value().unwrap_or_else(|| {
            println!(
                "Warning parsing JSDoc comment: Expected value for command {}",
                cmd.cmd
            );
            String::new()
        })
    }
}

struct ParsedCommand {
    /// The command, like @param or @example
    cmd: String,
    /// Attributes as key-value pairs, like `@example[text = type, another]`.
    /// Trimmed but otherwise arbitary (except for cannot contain equals sign
    /// or comma)
    attrs: HashMap<String, Option<String>>,
}

impl ParsedCommand {
    pub fn new<T: AsRef<str>>(cmd: T) -> Self {
        Self {
            cmd: cmd.as_ref().to_owned(),
            attrs: HashMap::new(),
        }
    }

    pub fn new_with<T: AsRef<str>>(cmd: T, attrs: HashMap<String, Option<String>>) -> Self {
        Self {
            cmd: cmd.as_ref().to_owned(),
            attrs,
        }
    }
}

struct Annotation {
    location: u32,
    link: UrlPath,
    class: String,
}

impl Annotation {
    pub fn from(value: &Entity, builder: &Builder, class: String) -> Option<Annotation> {
        Some(Self {
            location: value.get_range()?.get_start().get_file_location().offset,
            link: value
                .abs_docs_url(builder.config.clone())?
                .to_absolute(builder.config.clone()),
            class,
        })
    }

    pub fn from_end(value: &Entity, builder: &Builder, class: String) -> Option<Annotation> {
        Some(Self {
            location: value.get_range()?.get_end().get_file_location().offset - 1,
            link: value
                .abs_docs_url(builder.config.clone())?
                .to_absolute(builder.config.clone()),
            class,
        })
    }
}

fn annotate(base: Entity, annotations: &[Annotation]) -> Vec<Html> {
    let mut list = Vec::new();

    let mut prev: Option<Token> = None;
    for token in base.get_range().unwrap().tokenize() {
        let token_start = token.get_range().get_start().get_file_location();
        let token_end = token.get_range().get_end().get_file_location();

        // Add spaces if this is not the first token (trim from start and end)
        if let Some(prev) = prev {
            let prev_end = prev.get_range().get_end().get_file_location();

            let newlines = token_start.line - prev_end.line;

            let spaces =
                // If this token is on the same line as the previous one, spaces 
                // are the different between them
                if newlines == 0 {
                    token_start.column - prev_end.column
                }
                // Otherwise it's the difference from the start of this line
                else {
                    token_start.column
                };

            list.push(
                HtmlText::new("\n".repeat(newlines as usize) + &" ".repeat(spaces as usize)).into(),
            );
        }

        let classes: &[&str] = match token.get_kind() {
            TokenKind::Comment => &["comment"],
            TokenKind::Identifier => &["identifier"],
            TokenKind::Keyword => match token.get_spelling().as_str() {
                "true" | "false" | "this" => &["keyword", "value"],
                _ => &["keyword"],
            },
            TokenKind::Literal => &["literal"],
            TokenKind::Punctuation => &["punctuation"],
        };

        // Add link
        if let Some(a) = annotations
            .iter()
            .find(|a| token_start.offset <= a.location && a.location <= token_end.offset)
        {
            list.push(
                HtmlElement::new("a")
                    .with_classes(classes)
                    .with_class(&a.class)
                    .with_attr("href", a.link.clone())
                    .with_text(token.get_spelling())
                    .into(),
            );
        }
        // Add just the colorized token
        else {
            list.push(
                HtmlElement::new("span")
                    .with_classes(classes)
                    .with_text(token.get_spelling())
                    .into(),
            );
        }

        // Save current token as the previous in loop
        prev = Some(token);
    }

    list
}

fn print(entity: &Entity) {
    for child in entity.get_children() {
        println!(
            "{:?} :: {}:{}..{}:{} => {:?}",
            entity.get_kind(),
            child
                .get_range()
                .unwrap()
                .get_start()
                .get_file_location()
                .line,
            child
                .get_range()
                .unwrap()
                .get_start()
                .get_file_location()
                .column,
            child
                .get_range()
                .unwrap()
                .get_end()
                .get_file_location()
                .line,
            child
                .get_range()
                .unwrap()
                .get_end()
                .get_file_location()
                .column,
            child.get_kind(),
        );
    }
}

pub struct Example<'e> {
    builder: &'e Builder<'e>,
    data: String,
    analyze: bool,
}

impl<'e> Example<'e> {
    pub fn new(data: String, analyze: bool, builder: &'e Builder<'e>) -> Self {
        Self {
            builder,
            data,
            analyze,
        }
    }

    fn get_annotations(&self, entity: Entity<'e>) -> Vec<Annotation> {
        if !entity.is_in_main_file() {
            return Vec::new();
        }

        let mut res = Vec::new();

        if entity.get_kind() != EntityKind::TranslationUnit {
            print(&entity);
        }

        match entity.get_kind() {
            // Types
            EntityKind::TypeRef |
            // Templated types
            EntityKind::TemplateRef => {
                if let Some(p) = Annotation::from(&entity, self.builder, "class".into()) {
                    res.push(p);
                }
            },

            EntityKind::InclusionDirective => {
                if let Some(p) = Annotation::from(&entity, self.builder, "macro".into()) {
                    res.push(p);
                }
            },

            EntityKind::MacroExpansion => {
                if let Some(p) = Annotation::from(&entity, self.builder, "macro".into()) {
                    res.push(p);
                }
            },

            EntityKind::CallExpr => {
                if let Some(p) = Annotation::from_end(&entity.get_child(0).unwrap(), self.builder, "function".into()) {
                    res.push(p);
                }
            },

            _ => {},
        }

        for child in entity.get_children() {
            res.extend(self.get_annotations(child));
        }

        res
    }

    fn try_to_analyzed_html(&self) -> Result<Html, String> {
        // Create a temporary file to store the example's code in
        let mut num = 0;
        let path = loop {
            let path = self
                .builder
                .config
                .output_dir
                .join(format!("_example_{num}.cpp"));
            if !path.exists() {
                break path;
            }
            num += 1;
        };
        fs::write(&path, &self.data).map_err(|e| e.to_string())?;

        // Parse this file using builder's index to avoid reparsing everything
        let unit = self
            .builder
            .index
            .parser(&path)
            .arguments(self.builder.args)
            .parse()
            .map_err(|e| e.to_string())?;

        let res = HtmlElement::new("pre")
            .with_child(
                HtmlElement::new("code")
                    .with_classes(&["example"])
                    .with_children(annotate(
                        unit.get_entity(),
                        &self.get_annotations(unit.get_entity()),
                    )),
            )
            .into();

        // We don't really care if we can remove the file or not
        drop(fs::remove_file(path));

        Ok(res)
    }

    pub fn to_html(&self) -> Html {
        // Custom syntax highlighting with links
        if self.analyze && let Ok(sweet) = self.try_to_analyzed_html().inspect_err(|e|
            println!("Unable to parse example: {e}")
        ) {
            sweet
        }
        // Otherwise create a regular code block
        else {
            HtmlElement::new("pre")
                .with_child(HtmlElement::new("code")
                    .with_classes(&["example", "language-cpp"])
                    .with_text(&self.data)
                )
                .into()
        }
    }
}

pub struct JSDocComment<'e> {
    /// Description (duh)
    description: Option<String>,
    /// Parameters; specified with @param or @arg
    params: Vec<(String, String)>,
    /// Template parameters; specified with @tparam
    tparams: Vec<(String, String)>,
    /// Return value
    returns: Option<String>,
    /// What this throws
    throws: Option<String>,
    /// Refer to other doc item(s)
    see: Vec<String>,
    /// Notes about this item
    notes: Vec<String>,
    /// Warnings about this item
    warnings: Vec<String>,
    /// Item version
    version: Option<String>,
    /// When the item was added
    since: Option<String>,
    /// Examples
    examples: Vec<Example<'e>>,
    /// Reference to builder
    builder: &'e Builder<'e>,
}

impl<'e> JSDocComment<'e> {
    fn parse_mut(mut self, raw: String) -> Self {
        let mut lexer = CommentLexer::new(&raw);

        while let Some(cmd) = lexer.next_command() {
            match cmd.cmd.as_str() {
                "description" | "desc" | "brief" =>
                // Empty descriptions shouldn't result in warnings
                // This does make it so empty @description doesn't warn but eh
                // good enough
                {
                    self.description = lexer.next_value()
                }
                "param" | "arg" => self
                    .params
                    .push((lexer.param_for(&cmd), lexer.value_for(&cmd))),
                "tparam" | "targ" => self
                    .tparams
                    .push((lexer.param_for(&cmd), lexer.value_for(&cmd))),
                "return" | "returns" => self.returns = lexer.value_for(&cmd).into(),
                "throws" => self.throws = lexer.value_for(&cmd).into(),
                "see" => self.see.push(lexer.value_for(&cmd)),
                "note" => self.notes.push(lexer.value_for(&cmd)),
                "warning" | "warn" => self.warnings.push(lexer.value_for(&cmd)),
                "version" => self.version = lexer.value_for(&cmd).into(),
                "since" => self.since = lexer.value_for(&cmd).into(),
                "example" | "code" => self.examples.push(Example::new(
                    lexer.value_for(&cmd),
                    cmd.attrs.contains_key("flash"),
                    self.builder,
                )),
                // _ => println!("Warning parsing JSDoc comment: Unknown command {cmd}"),
                _ => {
                    // eat a value even though this is an unknown command
                    lexer.next_value();
                }
            }
        }

        self
    }

    pub fn new(builder: &'e Builder<'e>) -> Self {
        Self {
            description: None,
            params: Vec::new(),
            tparams: Vec::new(),
            returns: None,
            throws: None,
            see: Vec::new(),
            notes: Vec::new(),
            warnings: Vec::new(),
            version: None,
            since: None,
            examples: Vec::new(),
            builder,
        }
    }

    pub fn parse(raw: String, builder: &'e Builder<'e>) -> Self {
        Self::new(builder).parse_mut(raw)
    }

    pub fn to_html(&self, include_examples: bool) -> Html {
        HtmlList::new(vec![HtmlElement::new("div")
            .with_class("description")
            .with_child(
                HtmlElement::new("div")
                    .with_class("tags")
                    .with_child_opt(
                        self.version
                            .as_ref()
                            .map(|v| Html::p(format!("Version {v}"))),
                    )
                    .with_child_opt(self.since.as_ref().map(|v| Html::p(format!("Since {v}")))),
            )
            .with_child_opt(
                self.description
                    .as_ref()
                    .map(|d| fmt_markdown(
                        self.builder,
                        &fmt_autolinks(self.builder, d, None),
                        None::<fn(_) -> _>
                    )),
            )
            .with_child_opt(
                (!self.params.is_empty()).then_some(
                    HtmlElement::new("section")
                        .with_class("params")
                        .with_child(Html::span(&["title"], "Parameters"))
                        .with_child(
                            HtmlElement::new("div").with_class("grid").with_children(
                                self.params
                                    .iter()
                                    .flat_map(|param| {
                                        vec![Html::p(param.0.clone()), Html::div(param.1.clone())]
                                    })
                                    .collect(),
                            ),
                        ),
                ),
            )
            .with_child_opt(
                (!self.tparams.is_empty()).then_some(
                    HtmlElement::new("section")
                        .with_classes(&["params", "template"])
                        .with_child(Html::span(&["title"], "Template parameters"))
                        .with_child(
                            HtmlElement::new("div").with_class("grid").with_children(
                                self.tparams
                                    .iter()
                                    .flat_map(|tparam| {
                                        vec![Html::p(tparam.0.clone()), Html::div(tparam.1.clone())]
                                    })
                                    .collect(),
                            ),
                        ),
                ),
            )
            .with_child_opt(self.returns.as_ref().map(|ret| {
                HtmlElement::new("section")
                    .with_classes(&["params", "returns", "grid"])
                    .with_child(Html::span(&["title"], "Return value"))
                    .with_child(Html::div(ret.clone()))
            }))
            .with_child_opt(self.throws.as_ref().map(|ret| {
                HtmlElement::new("section")
                    .with_classes(&["params", "throws", "grid"])
                    .with_child(Html::span(&["title"], "Exceptions"))
                    .with_child(Html::div(ret.clone()))
            }))
            // todo: see
            .with_children(
                self.notes
                    .iter()
                    .map(|note| {
                        HtmlElement::new("section")
                            .with_class("note")
                            .with_child(Html::span(&["title"], "Note"))
                            .with_child(Html::div(note.clone()))
                            .into()
                    })
                    .collect(),
            )
            .with_children(
                self.warnings
                    .iter()
                    .map(|warning| {
                        HtmlElement::new("section")
                            .with_class("warning")
                            .with_child(Html::span(&["title"], "Warning"))
                            .with_child(Html::div(warning.clone()))
                            .into()
                    })
                    .collect(),
            )
            .with_children(if include_examples {
                self.examples
                    .iter()
                    .map(|example| example.to_html())
                    .collect()
            } else {
                Vec::new()
            })
            .into()])
        .into()
    }

    pub fn examples(&self) -> &Vec<Example> {
        &self.examples
    }
}

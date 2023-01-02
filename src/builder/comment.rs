
use std::{iter::Peekable, str::Chars};

use crate::html::{Html, HtmlElement, HtmlText};

struct CommentLexer<'s> {
    raw: Peekable<Chars<'s>>,
}

impl<'s> CommentLexer<'s> {
    pub fn new(raw: &'s String) -> Self {
        Self {
            raw: raw.trim_end_matches("*/").trim_start_matches("/*").chars().peekable(),
        }
    }

    fn skip_until<P: FnMut(char) -> bool>(&mut self, mut pred: P) {
        while self.raw.peek().is_some_and(|c| pred(*c)) {
            self.raw.next();
        }
    }

    fn skip_whitespace(&mut self) {
        self.skip_until(|c| c.is_whitespace());
    }

    fn skip_line(&mut self) {
        self.skip_until(|c| c.is_whitespace() || c == '*');
    }

    fn eat_until<P: FnMut(char) -> bool>(&mut self, mut pred: P) -> Option<String> {
        let mut res = String::new();
        while let Some(c) = self.raw.peek().map(|c| *c) {
            if pred(c) {
                break;
            }
            // On newlines, skip whitespace and the next line's starting star 
            // if there is one, compressing everything to one space
            if c == '\n' {
                self.skip_line();
                res.push(' ');
            }
            else {
                self.raw.next();
                res.push(c);
            }
        }
        (res.len() > 0).then_some(res)
    }

    fn eat_word(&mut self) -> Option<String> {
        self.eat_until(|c| c.is_whitespace())
    }
    
    pub fn next_command(&mut self) -> Option<String> {
        // Skip whitespace
        self.skip_line();

        // todo: handle escaped @ symbol

        match self.raw.peek()? {
            '@' => {
                // Consume param symbol
                self.raw.next();
                self.eat_word()
            }
            _ => Some("description".into()),
        }
    }

    pub fn next_param(&mut self) -> Option<String> {
        self.skip_whitespace();
        self.eat_word()
    }

    pub fn next_value(&mut self) -> Option<String> {
        self.skip_whitespace();
        self.eat_until(|c| c == '@').map(|s| s.trim().to_owned())
    }

    pub fn param_for(&mut self, cmd: &String) -> String {
        self.next_param().unwrap_or_else(|| {
            println!("Warning parsing JSDoc comment: Expected parameter for command {cmd}");
            String::new()
        })
    }

    pub fn value_for(&mut self, cmd: &String) -> String {
        self.next_value().unwrap_or_else(|| {
            println!("Warning parsing JSDoc comment: Expected value for command {cmd}");
            String::new()
        })
    }
}

pub struct JSDocComment {
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
    examples: Vec<String>,
}

impl JSDocComment {
    fn parse_mut(mut self, raw: String) -> Self {
        let mut lexer = CommentLexer::new(&raw);

        while let Some(cmd) = lexer.next_command() {
            match cmd.as_str() {
                "description" | "desc" => 
                    self.description = lexer.value_for(&cmd).into(),
                "param" | "arg" => 
                    self.params.push((
                        lexer.param_for(&cmd),
                        lexer.value_for(&cmd),
                    )),
                "tparam" | "targ" =>
                    self.tparams.push((
                        lexer.param_for(&cmd),
                        lexer.value_for(&cmd),
                    )),
                "return" | "returns" => self.returns = lexer.value_for(&cmd).into(),
                "throws" => self.throws = lexer.value_for(&cmd).into(),
                "see" => self.see.push(lexer.value_for(&cmd)),
                "note" => self.notes.push(lexer.value_for(&cmd)),
                "warning" | "warn" => self.warnings.push(lexer.value_for(&cmd)),
                "version" => self.version = lexer.value_for(&cmd).into(),
                "since" => self.since = lexer.value_for(&cmd).into(),
                "example" | "code" => self.examples.push(lexer.value_for(&cmd)),
                // _ => println!("Warning parsing JSDoc comment: Unknown command {cmd}"),
                _ => {
                    // eat a value even though this is an unknown command
                    lexer.next_value();
                },
            }
        }

        self
    }

    pub fn new() -> Self {
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
        }
    }

    pub fn parse(raw: String) -> Self {
        Self::new().parse_mut(raw)
    }

    pub fn to_html(&self) -> Html {
        HtmlElement::new("div")
            .with_class("description")
            .with_child(HtmlElement::new("div")
                .with_class("tags")
                .with_child_opt(self.version.as_ref().map(|v| Html::p(format!("Version {v}"))))
                .with_child_opt(self.since.as_ref().map(|v| Html::p(format!("Since {v}"))))
            )
            .with_child_opt(self.description.as_ref().map(HtmlText::new))
            .with_child_opt((!self.params.is_empty()).then_some(
                HtmlElement::new("section")
                    .with_class("params")
                    .with_children(self.params.iter().flat_map(|param|
                        vec![
                            Html::p(param.0.clone()),
                            Html::div(param.1.clone()),
                        ]
                    ).collect())
            ))
            .with_child_opt((!self.tparams.is_empty()).then_some(
                HtmlElement::new("section")
                    .with_classes(&["params", "template"])
                    .with_children(self.tparams.iter().flat_map(|tparam|
                        vec![
                            Html::p(tparam.0.clone()),
                            Html::div(tparam.1.clone()),
                        ]
                    ).collect())
            ))
            .with_child_opt(self.returns.as_ref().map(|ret|
                HtmlElement::new("section")
                    .with_classes(&["params", "returns"])
                    .with_children(
                        vec![
                            Html::p("Returns"),
                            Html::div(ret.clone()),
                        ]
                    )
            ))
            .with_child_opt(self.throws.as_ref().map(|ret|
                HtmlElement::new("section")
                    .with_classes(&["params", "throws"])
                    .with_children(
                        vec![
                            Html::p("Throws"),
                            Html::div(ret.clone()),
                        ]
                    )
            ))
            // todo: see
            .with_children(self.notes.iter().map(|note|
                HtmlElement::new("section")
                    .with_class("note")
                    .with_children(
                        vec![
                            Html::p("Note"),
                            Html::div(note.clone()),
                        ]
                    )
                    .into()
            ).collect())
            .with_children(self.warnings.iter().map(|warning|
                HtmlElement::new("section")
                    .with_class("warning")
                    .with_children(
                        vec![
                            Html::p("Warning"),
                            Html::div(warning.clone()),
                        ]
                    )
                    .into()
            ).collect())
            .with_children(self.examples.iter().map(|example|
                HtmlElement::new("code")
                    .with_class("example")
                    .with_text(example)
                    .into()
            ).collect())
            .into()
    }
}

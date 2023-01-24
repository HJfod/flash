use std::collections::HashMap;

pub trait GenHtml: Into<Html> {
    fn gen_html(self) -> String;
}

pub enum Html {
    /// A HTML element with a tag, attributes, and children
    Element(HtmlElement),
    /// Text inside a HTML element
    Text(HtmlText),
    /// A list of HTML elements. Used to return a bunch of stuff with no root
    /// element connecting them
    List(HtmlList),
    /// Raw HTML data
    Raw(String),
}

impl Html {
    /// Helper for creating a <p> element
    pub fn p<T: AsRef<str>>(text: T) -> Html {
        HtmlElement::new("p").with_text(text).into()
    }

    pub fn div<T: AsRef<str>>(text: T) -> Html {
        HtmlElement::new("div").with_text(text).into()
    }

    pub fn span(classes: &[&str], text: &str) -> Html {
        HtmlElement::new("span")
            .with_classes(classes)
            .with_text(text)
            .into()
    }

    pub fn feather(icon: &str) -> Html {
        HtmlElement::new("i").with_attr("data-feather", icon).into()
    }
}

impl GenHtml for Html {
    fn gen_html(self) -> String {
        match self {
            Self::Element(e) => e.gen_html(),
            Self::Text(t) => t.gen_html(),
            Self::List(l) => l.gen_html(),
            Self::Raw(s) => s,
        }
    }
}

pub struct HtmlElement {
    tag: String,
    classes: Vec<String>,
    attributes: HashMap<String, String>,
    children: Vec<Html>,
}

#[allow(unused)]
impl HtmlElement {
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.into(),
            classes: Vec::new(),
            attributes: HashMap::new(),
            children: Vec::new(),
        }
    }

    pub fn has_class(&self, name: &str) -> bool {
        self.classes.iter().any(|cls| cls == name)
    }

    pub fn with_class(mut self, name: &str) -> Self {
        self.classes.push(name.into());
        self
    }

    pub fn with_classes(mut self, classes: &[&str]) -> Self {
        self.classes.extend(classes.iter().map(|s| s.to_string()));
        self
    }

    pub fn with_class_opt(self, name: Option<&str>) -> Self {
        if let Some(name) = name {
            self.with_class(name)
        } else {
            self
        }
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn add_child<T: GenHtml>(&mut self, child: T) {
        self.children.push(child.into());
    }

    pub fn add_child_opt<T: GenHtml>(&mut self, child: Option<T>) {
        if let Some(child) = child {
            self.children.push(child.into());
        }
    }

    pub fn with_child<T: GenHtml>(mut self, child: T) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn with_children(mut self, children: Vec<Html>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn with_child_opt<T: GenHtml>(mut self, child: Option<T>) -> Self {
        if let Some(child) = child {
            self.children.push(child.into());
        }
        self
    }

    pub fn with_text<T: AsRef<str>>(mut self, text: T) -> Self {
        self.children.push(HtmlText::new(text).into());
        self
    }

    pub fn has_attr(&self, attr: &str) -> bool {
        self.attributes.contains_key(attr)
    }

    pub fn attr_mut(&mut self, attr: &str) -> &mut String {
        self.attributes.entry(attr.into()).or_insert(String::new())
    }

    pub fn with_attrs(mut self, attrs: &Vec<(String, String)>) -> Self {
        for (attr, value) in attrs {
            *self.attr_mut(attr) = value.to_string();
        }
        self
    }

    pub fn with_attr<V: ToString>(mut self, attr: &str, value: V) -> Self {
        *self.attr_mut(attr) = value.to_string();
        self
    }

    pub fn with_attr_opt<V: ToString>(self, attr: &str, value: Option<V>) -> Self {
        if let Some(value) = value {
            self.with_attr(attr, value)
        } else {
            self
        }
    }
}

impl GenHtml for HtmlElement {
    fn gen_html(self) -> String {
        format!(
            "<{tag} {classes} {attrs}>{children}</{tag}>",
            tag = self.tag,
            classes = self.classes.is_empty().then_some(String::new()).unwrap_or(
                format!("class=\"{}\"", self.classes.join(" "))
            ),
            attrs = self
                .attributes
                .iter()
                .map(|(k, v)| match k.as_str() {
                    "onclick" => format!("{k}=\"{v}\""),
                    _ => format!("{k}=\"{}\"", v.escape_default()),
                })
                .collect::<Vec<_>>()
                .join(" "),
            children = self
                .children
                .into_iter()
                .map(|c| c.gen_html())
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl From<HtmlElement> for Html {
    fn from(val: HtmlElement) -> Self {
        Html::Element(val)
    }
}

pub struct HtmlText {
    content: String,
}

impl HtmlText {
    pub fn new<T: AsRef<str>>(content: T) -> Self {
        Self {
            content: content.as_ref().into(),
        }
    }
}

impl GenHtml for HtmlText {
    fn gen_html(self) -> String {
        sanitize_html(&self.content)
    }
}

impl From<HtmlText> for Html {
    fn from(val: HtmlText) -> Self {
        Html::Text(val)
    }
}

pub struct HtmlList {
    list: Vec<Html>,
}

impl HtmlList {
    pub fn new(list: Vec<Html>) -> Self {
        Self { list }
    }
}

impl GenHtml for HtmlList {
    fn gen_html(self) -> String {
        self.list
            .into_iter()
            .map(|i| i.gen_html())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl From<HtmlList> for Html {
    fn from(val: HtmlList) -> Self {
        Html::List(val)
    }
}

fn sanitize_html(html: &str) -> String {
    html.replace('<', "&lt;").replace('>', "&gt;")
}

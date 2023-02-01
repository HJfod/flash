
use super::builder::Builder;
use super::shared::fmt_emoji;
use super::traits::Entry;
use crate::html::{Html, HtmlElement, HtmlText};
use crate::url::UrlPath;
use pulldown_cmark::{CowStr, Event, Tag, LinkType};
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Metadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
}

fn parse_markdown_metadata<'a>(doc: &'a str) -> (&'a str, Option<Metadata>) {
    // if the document has no metadata just parse it as markdown
    if !doc.trim_start().starts_with("---") {
        return (doc, None);
    }

    let doc = doc.trim_start().strip_prefix("---").unwrap();

    // make sure metadata ends properly
    let Some(metadata_end) = doc.find("---") else {
        return (doc, None);
    };
    let metadata_str = &doc[..metadata_end];

    // parse metadata
    (
        &doc[metadata_end + 3..],
        serde_yaml::from_str(metadata_str).expect("Invalid metadata in markdown")
    )
}

#[allow(clippy::ptr_arg)]
pub fn fmt_markdown<F: Fn(UrlPath) -> Option<UrlPath>>(
    builder: &Builder, text: &str, url_fixer: Option<F>
) -> Html {
    // skip metadata
    let (text, _) = parse_markdown_metadata(text);
    let parser = pulldown_cmark::Parser::new_ext(
        text, pulldown_cmark::Options::all()
    );

    let mut content = String::new();
    pulldown_cmark::html::push_html(
        &mut content,
        parser.map(|event| match event {
            Event::Text(t) => Event::Text(CowStr::Boxed(Box::from(
                fmt_emoji(&t).as_str()
            ))),
            // fix urls to point to root
            Event::Start(tag) => match tag {
                Tag::Link(ty, ref dest, ref title) | Tag::Image(ty, ref dest, ref title) => {
                    let mut new_dest;
                    if ty == LinkType::Inline 
                        && dest.starts_with("/")
                        && let Some(ref url_fixer) = url_fixer
                    {
                        let url = UrlPath::new_with_path(
                            dest.split("/").map(|s| s.to_string()).collect()
                        );
                        if let Some(url) = url_fixer(url) {
                            new_dest = url.to_string();
                        }
                        else {
                            new_dest = dest.to_string();
                        }
                    }
                    else {
                        new_dest = dest.to_string();
                    }

                    // make the url absolute in any case if it starts with /
                    if dest.starts_with("/") && let Ok(dest) = UrlPath::parse(&new_dest) {
                        new_dest = dest.to_absolute(builder.config.clone()).to_string();
                    }

                    // return fixed url
                    if matches!(tag, Tag::Link(_, _, _)) {
                        Event::Start(Tag::Link(
                            ty,
                            CowStr::Boxed(Box::from(new_dest)),
                            title.to_owned()
                        ))
                    }
                    else {
                        Event::Start(Tag::Image(
                            ty,
                            CowStr::Boxed(Box::from(new_dest)),
                            title.to_owned()
                        ))
                    }
                }
                _ => Event::Start(tag)
            }
            _ => event,
        }),
    );

    HtmlElement::new("div")
        .with_class("text")
        .with_child(Html::Raw(content))
        .into()
}

#[allow(clippy::ptr_arg)]
pub fn extract_metadata_from_md(text: &String, default_title: Option<String>) -> Option<Metadata> {
    let (text, metadata) = parse_markdown_metadata(text);

    // if the metadata provided the title, no need to parse the doc for it
    if metadata.is_some() && metadata.as_ref().unwrap().title.is_some() {
        return metadata;
    }

    // otherwise parse doc and use first header as title
    let mut parser = pulldown_cmark::Parser::new_ext(text, pulldown_cmark::Options::all());

    let name = parser.next()?;
    let Event::Start(tag) = name else { return None };
    let Tag::Heading(_, _, _) = tag else { return None };

    let mut res = String::new();

    while match parser.next() {
        Some(ev) => match ev {
            Event::End(tag) => !matches!(tag, Tag::Heading(_, _, _)),
            Event::Text(text) => {
                res.push_str(&text);
                true
            }
            _ => true,
        },
        None => false,
    } {}

    // if some metadata was found, set the title
    if let Some(mut metadata) = metadata {
        metadata.title = (!res.is_empty()).then_some(res).or(default_title);
        Some(metadata)
    }
    // otherwise only return Some if a title was found
    else {
        if res.is_empty() {
            if let Some(title) = default_title {
                Some(Metadata {
                    title: Some(title),
                    description: None,
                    icon: None,
                })
            }
            else {
                None
            }
        }
        else {
            Some(Metadata {
                title: Some(res),
                description: None,
                icon: None,
            })
        }
    }
}

pub fn output_tutorial<'e, T: Entry<'e>>(
    entry: &T,
    builder: &Builder,
    content: &str,
    links: Html,
) -> Vec<(&'static str, Html)> {
    vec![
        ("title", HtmlText::new(entry.name()).into()),
        (
            "content",
            fmt_markdown(
                builder,
                &content,
                Some(|url: UrlPath| {
                    Some(url.remove_extension(".md"))
                }),
            ),
        ),
        ("links", links),
    ]
}

use crate::config::Config;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use serde::{de::Visitor, Deserialize};
use std::{fmt::Display, path::PathBuf, sync::Arc};

// The URL crate doesn't support paths like /some/file, it needs the protocol and hostname
// (which is undesirable for Flash as docs links are /docs/namespace/entity)

pub const URL_RESERVED: &AsciiSet = &CONTROLS
    // reserved characters
    .add(b'!')
    .add(b'#')
    .add(b'$')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b']')
    // non-reserved ascii characters that should be escaped
    .add(b'<')
    .add(b'>')
    .add(b' ')
    .add(b'{')
    .add(b'}')
    .add(b'\\')
    .add(b'|')
    .add(b'"');

#[derive(Debug, Clone, PartialEq)]
pub struct UrlPath {
    parts: Vec<String>,
}

impl UrlPath {
    pub fn new() -> Self {
        Self { parts: Vec::new() }
    }

    pub fn new_with_path(parts: Vec<String>) -> Self {
        Self { parts }.clean()
    }

    pub fn parse(url: &str) -> Result<Self, String> {
        Ok(UrlPath::new_with_path(
            url.split("/").map(|s| s.to_owned()).collect(),
        ))
    }

    fn clean(mut self) -> Self {
        // based on https://github.com/ivanceras/url_path/blob/ffdf3dd883ed4a9395eeb9cf9b1990539508a7a6/src/lib.rs
        let mut filtered = Vec::new();
        self.parts
            .iter()
            .filter_map(|p| (!p.is_empty() || p != ".").then_some(p.to_owned()))
            .for_each(|p| {
                if p == ".." {
                    filtered.pop();
                } else {
                    filtered.push(p);
                }
            });
        self.parts = filtered;
        self
    }

    pub fn join<T: AsRef<UrlPath>>(&self, other: T) -> Self {
        let mut buf = self.clone();
        buf.parts.extend(other.as_ref().parts.clone());
        buf.clean()
    }

    /// Strip prefix. If prefix is not a prefix of this URL, nothing happens
    pub fn strip_prefix<T: AsRef<UrlPath>>(&self, prefix: T) -> Self {
        // Make sure prefix is shorter or as long as path
        if self.parts.len() >= prefix.as_ref().parts.len() {
            if self.parts[0..prefix.as_ref().parts.len()] == prefix.as_ref().parts {
                return UrlPath::new_with_path(self.parts[prefix.as_ref().parts.len()..].into());
            }
        }
        self.clone()
    }

    pub fn url_safe_parts(&self) -> Vec<String> {
        self.parts
            .iter()
            .map(|p| utf8_percent_encode(p, URL_RESERVED).to_string())
            .collect()
    }

    pub fn file_name(&self) -> Option<String> {
        self.url_safe_parts().last().map(|s| s.to_owned())
    }

    pub fn raw_file_name(&self) -> Option<String> {
        self.parts.last().map(|s| s.to_owned())
    }

    pub fn to_raw_string(&self) -> String {
        self.parts.join("/")
    }

    pub fn to_pathbuf(&self) -> PathBuf {
        PathBuf::from_iter(&self.url_safe_parts())
    }

    pub fn to_absolute(&self, config: Arc<Config>) -> Self {
        config
            .output_url
            .as_ref()
            .unwrap_or(&UrlPath::new())
            .join(self)
    }
}

struct UrlVisitor;

impl<'de> Visitor<'de> for UrlVisitor {
    type Value = UrlPath;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an URL path")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        UrlPath::try_from(v).map_err(|e| serde::de::Error::custom(e))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        UrlPath::try_from(v).map_err(|e| serde::de::Error::custom(e))
    }
}

impl<'de> Deserialize<'de> for UrlPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_string(UrlVisitor)
    }
}

// Idk how to do this better :(

impl AsRef<UrlPath> for UrlPath {
    fn as_ref(&self) -> &UrlPath {
        self
    }
}

impl TryFrom<&PathBuf> for UrlPath {
    type Error = String;

    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        Ok(UrlPath::new_with_path(
            value
                .components()
                .map(|p| {
                    p.as_os_str()
                        .to_str()
                        .map(|s| s.to_string())
                        .ok_or("Expected UTF-8".to_owned())
                })
                .collect::<Result<_, _>>()?,
        ))
    }
}

impl TryFrom<&str> for UrlPath {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        UrlPath::parse(value)
    }
}

impl TryFrom<String> for UrlPath {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        UrlPath::parse(&value)
    }
}

impl TryFrom<&String> for UrlPath {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        UrlPath::parse(&value)
    }
}

impl Display for UrlPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("/{}", &self.url_safe_parts().join("/")))
    }
}

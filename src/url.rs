
use std::{path::PathBuf, fmt::Display};
use serde::{Deserialize, de::Visitor};
use crate::config::Config;

#[derive(Debug, Clone, PartialEq)]
pub struct UrlPath {
    protocol: Option<String>,
    host: Option<String>,
    parts: Vec<String>,
}

impl UrlPath {
    pub fn new() -> Self {
        Self {
            protocol: None,
            host: None,
            parts: Vec::new()
        }
    }

    pub fn new_with_path(parts: Vec<String>) -> Self {
        Self {
            protocol: None,
            host: None,
            parts
        }.clean()
    }

    pub fn parse(url: String) -> Result<Self, String> {
        
    }

    fn clean(mut self) -> Self {
        // based on https://github.com/ivanceras/url_path/blob/ffdf3dd883ed4a9395eeb9cf9b1990539508a7a6/src/lib.rs
        let mut filtered = Vec::new();
        self.parts
            .iter()
            .filter_map(|p| (!p.is_empty() || p != ".").then_some(p.to_owned()))
            .for_each(|p| if p == ".." {
                filtered.pop();
            } else {
                filtered.push(p);
            });
        self.parts = filtered;
        self
    }

    pub fn join<T: AsRef<UrlPath>>(&self, other: T) -> Self {
        let mut buf = self.clone();
        buf.parts.extend(other.as_ref().parts.clone());
        buf.clean()
    }

    pub fn protocol(&self) -> &Option<String> {
        &self.protocol
    }

    pub fn host(&self) -> &Option<String> {
        &self.host
    }

    pub fn file_name(&self) -> Option<&String> {
        self.parts.last()
    }

    pub fn to_raw_string(&self) -> String {
        self.parts.join("/")
    }

    pub fn to_pathbuf(&self) -> PathBuf {
        PathBuf::from_iter(&self.parts)
    }

    pub fn to_absolute(&self, config: &Config) -> Self {
        UrlPath::from(config.output_url.as_ref().unwrap_or(&UrlPath::new())).join(self)
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
            E: serde::de::Error
    {
        Ok(UrlPath::from(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
    {
        Ok(UrlPath::from(v))
    }
}

impl<'de> Deserialize<'de> for UrlPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>
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

impl TryFrom<PathBuf> for UrlPath {
    type Error = String;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Ok(UrlPath::new_with_path(
            value
                .components()
                .map(|p| p
                    .as_os_str()
                    .to_str()
                    .map(|s| s.to_string()).ok_or("Expected UTF-8".to_owned())
                )
                .collect::<Result<_, _>>()?
        ))
    }
}

impl TryFrom<String> for UrlPath {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        
    }
}

impl Display for UrlPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("/{}", &self.parts.join("/")))
    }
}

use std::{path::PathBuf, fmt::Display};
use crate::config::Config;

#[derive(Debug)]
pub struct Url {
    parts: Vec<String>,
}

impl Url {
    pub fn new(parts: Vec<String>) -> Self {
        Self {
            parts,
        }
    }

    pub fn join<T: AsRef<Url>>(&mut self, other: T) -> &mut Self {
        self.parts.extend(other.as_ref().parts);
        self
    }

    pub fn to_absolute(&self, config: &Config) -> Self {
        *Url::from(config.relative_output_dir.unwrap_or(PathBuf::new())).join(self)
    }
}

impl AsRef<Url> for &Url {
    fn as_ref(&self) -> &Url {
        self
    }
}

impl From<PathBuf> for Url {
    fn from(value: PathBuf) -> Self {
        Url::new(value
            .components()
            // todo: make sure only url-safe characters are included
            .map(|p| p.as_os_str().to_string_lossy().to_string())
            .collect()
        )
    }
}

impl From<&str> for Url {
    fn from(value: &str) -> Self {
        Url::new(value.split(&['/', '\\']).map(|s| s.to_owned()).collect())
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.parts.join("/"))
    }
}

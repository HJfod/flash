use flash_macros::decl_config;
use glob::glob;
use serde::{Deserialize, Deserializer};
use std::{fs, path::PathBuf, sync::Arc};

use crate::url::UrlPath;

fn parse_template<'de, D>(deserializer: D) -> Result<Arc<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Arc::from(
        fs::read_to_string(PathBuf::deserialize(deserializer)?)
        .map_err(serde::de::Error::custom)?
    ))
}

fn parse_glob<'de, D>(deserializer: D) -> Result<Vec<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Vec::<PathBuf>::deserialize(deserializer)?
        .iter()
        .flat_map(|src| {
            glob(src.to_str().unwrap())
                .unwrap_or_else(|_| panic!("Invalid glob pattern {}", src.to_str().unwrap()))
                .map(|g| g.unwrap())
        })
        .collect())
}

macro_rules! default_template {
    ($name: expr) => {
        Arc::from(include_str!($name).to_string())
    };
}

macro_rules! default_scripts {
    () => {
        Vec::new(),
    };

    (@ $name: expr) => {
        Script {
            name: $name.into(),
            content: default_template!(concat!("../templates/", $name)),
        }
    };

    ($name: expr $(, $rest: expr)*) => {
        vec![default_scripts!(@ $name), $(default_scripts!(@ $rest)),*]
    };
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Script {
    pub name: String,
    #[serde(deserialize_with = "parse_template")]
    pub content: Arc<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BrowserRoot {
    pub path: UrlPath,
    pub include_prefix: UrlPath,
    pub name: String,
}

decl_config! {
    struct Config {
        project {
            name: String,
            version: String,
            repository?: String,
        },
        docs {
            include: Vec<PathBuf> as parse_glob,
            exclude: Vec<PathBuf> as parse_glob = Vec::new(),
            tree?: String,
        },
        browser {
            roots: Vec<Arc<BrowserRoot>> = Vec::new(),
        },
        run? {
            prebuild?: Vec<String>,
        },
        analysis {
            compile_args: Vec<String> = Vec::new(),
        },
        cmake? {
            config_args?: Vec<String>,
            build_args?: Vec<String>,
            build: bool = false,
            infer_args_from: PathBuf,
        },
        templates {
            class:   Arc<String> as parse_template = default_template!("../templates/class.html"),
            struct_: Arc<String> as parse_template = default_template!("../templates/struct.html"),
            index:   Arc<String> as parse_template = default_template!("../templates/index.html"),
            head:    Arc<String> as parse_template = default_template!("../templates/head.html"),
            nav:     Arc<String> as parse_template = default_template!("../templates/nav.html"),
            file:    Arc<String> as parse_template = default_template!("../templates/file.html"),
            page:    Arc<String> as parse_template = default_template!("../templates/page.html"),
        },
        scripts {
            css: Vec<Script> = default_scripts!("default.css", "nav.css", "content.css", "themes.css"),
            js:  Vec<Script> = default_scripts!("script.js"),
        },
        let input_dir: PathBuf,
        let output_dir: PathBuf,
        let output_url: Option<UrlPath>,
    }
}

impl Config {
    pub fn parse(
        input_dir: PathBuf,
        output_dir: PathBuf,
        output_url: Option<UrlPath>,
    ) -> Result<Arc<Config>, String> {
        let mut config: Config = toml::from_str(
            &fs::read_to_string(input_dir.join("flash.toml"))
                .map_err(|e| format!("Unable to read flash.toml: {e}"))?,
        )
        .map_err(|e| format!("Unable to parse config: {e}"))?;

        config.input_dir = input_dir;
        config.output_dir = output_dir;
        config.output_url = output_url;
        Ok(Arc::from(config))
    }

    pub fn filtered_includes(&self) -> Vec<&PathBuf> {
        self.docs
            .include
            .iter()
            .filter(|p| !self.docs.exclude.contains(p))
            .collect()
    }
}

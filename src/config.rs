use std::{fs, path::PathBuf};
use glob::glob;
use serde::{Deserialize, Deserializer};
use flash_macros::decl_config;

use crate::url::UrlPath;

fn parse_template<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    fs::read_to_string(PathBuf::deserialize(deserializer)?).map_err(serde::de::Error::custom)
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

macro_rules! default_script {
    ($name: literal) => {
        Script {
            name: $name.into(),
            content: include_str!(concat!("../templates/", $name)).into(),
        }
    };
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Script {
    pub name: String,
    #[serde(deserialize_with = "parse_template")]
    pub content: String,
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
            roots: Vec<BrowserRoot> = Vec::new(),
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
            class: String as parse_template = include_str!("../templates/class.html").to_string(),
            index: String as parse_template = include_str!("../templates/index.html").to_string(),
            head:  String as parse_template = include_str!("../templates/head.html").to_string(),
            nav:   String as parse_template = include_str!("../templates/nav.html").to_string(),
            file:  String as parse_template = include_str!("../templates/file.html").to_string(),
            page:  String as parse_template = include_str!("../templates/page.html").to_string(),
        },
        scripts {
            css: Vec<Script> = vec![default_script!("default.css")],
            js:  Vec<Script> = vec![default_script!("script.js")],
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
    ) -> Result<Config, String> {
        let mut config: Config = toml::from_str(
            &fs::read_to_string(input_dir.join("flash.toml"))
                .map_err(|e| format!("Unable to read flash.toml: {e}"))?,
        )
        .map_err(|e| format!("Unable to parse config: {e}"))?;

        config.input_dir = input_dir;
        config.output_dir = output_dir;
        config.output_url = output_url;
        Ok(config)
    }

    pub fn filtered_includes(&self) -> Vec<&PathBuf> {
        self.docs
            .include
            .iter()
            .filter(|p| !self.docs.exclude.contains(p))
            .collect()
    }
}

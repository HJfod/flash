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
            .map_err(serde::de::Error::custom)?,
    ))
}

fn parse_sources<'de, D>(deserializer: D) -> Result<Vec<Arc<Source>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Vec::<RawSource>::deserialize(deserializer)?
        .into_iter()
        .map(|src| Arc::from(Source::from_raw(src).unwrap()))
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

pub struct Source {
    pub name: String,
    pub dir: UrlPath,
    pub include: Vec<PathBuf>,
    pub strip_include_prefix: Option<PathBuf>,
}

impl Source {
    pub fn from_raw(src: RawSource) -> Result<Source, String> {
        let exclude = src
            .exclude
            .into_iter()
            .map(|p| src.dir.to_pathbuf().join(p))
            .flat_map(|src| {
                glob(src.to_str().unwrap())
                    .unwrap_or_else(|_| panic!("Invalid glob pattern {}", src.to_str().unwrap()))
                    .map(|g| g.unwrap())
            })
            .collect::<Vec<_>>();

        let include = src
            .include
            .into_iter()
            .map(|p| src.dir.to_pathbuf().join(p))
            .flat_map(|src| {
                glob(src.to_str().unwrap())
                    .unwrap_or_else(|_| panic!("Invalid glob pattern {}", src.to_str().unwrap()))
                    .map(|g| g.unwrap())
            })
            .filter(|p| !exclude.contains(p))
            .collect::<Vec<_>>();

        Ok(Self {
            name: src.name,
            dir: src.dir,
            strip_include_prefix: src.strip_include_prefix,
            include,
        })
    }

    pub fn include_prefix(&self) -> UrlPath {
        UrlPath::try_from(
            self.strip_include_prefix
                .as_ref()
                .unwrap_or(&PathBuf::new()),
        )
        .unwrap_or(UrlPath::new())
    }
}

decl_config! {
    struct Script {
        name: String,
        content: Arc<String> as parse_template,
    }

    struct RawSource {
        name: String,
        dir: UrlPath,
        include: Vec<PathBuf>,
        exclude: Vec<PathBuf> = Vec::new(),
        strip_include_prefix?: PathBuf,
    }

    struct Config {
        project {
            name: String,
            version: String,
            repository?: String,
            tree?: String,
        },
        sources: Vec<Arc<Source>> as parse_sources,
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
            build_dir: String = String::from("build"),
            infer_args_from: PathBuf,
        },
        templates {
            class:    Arc<String> as parse_template = default_template!("../templates/class.html"),
            struct_:  Arc<String> as parse_template = default_template!("../templates/struct.html"),
            function: Arc<String> as parse_template = default_template!("../templates/function.html"),
            index:    Arc<String> as parse_template = default_template!("../templates/index.html"),
            head:     Arc<String> as parse_template = default_template!("../templates/head.html"),
            nav:      Arc<String> as parse_template = default_template!("../templates/nav.html"),
            file:     Arc<String> as parse_template = default_template!("../templates/file.html"),
            page:     Arc<String> as parse_template = default_template!("../templates/page.html"),
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

    pub fn all_includes(&self) -> Vec<PathBuf> {
        self.sources
            .iter()
            .flat_map(|src| src.include.clone())
            .collect()
    }
}

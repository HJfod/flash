use std::{fs, path::PathBuf};

use glob::glob;
use serde::{Deserialize, Deserializer};

fn default_class_template() -> String {
    include_str!("../templates/class.html").into()
}

fn default_index_template() -> String {
    include_str!("../templates/index.html").into()
}

fn default_head_template() -> String {
    include_str!("../templates/head.html").into()
}

fn default_nav_template() -> String {
    include_str!("../templates/nav.html").into()
}

fn default_file_template() -> String {
    include_str!("../templates/file.html").into()
}

fn default_css() -> String {
    include_str!("../templates/default.css").into()
}

fn default_js() -> String {
    include_str!("../templates/script.js").into()
}

const fn cmake_build_default() -> bool {
    false
}

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

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CMakeConfig {
    pub config_args: Option<Vec<String>>,
    pub build_args: Option<Vec<String>>,
    #[serde(default = "cmake_build_default")]
    pub build: bool,
    pub infer_args_from: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BrowserRoot {
    pub path: PathBuf,
    pub name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BrowserConfig {
    #[serde(default = "Vec::new")]
    pub roots: Vec<BrowserRoot>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DocsConfig {
    #[serde(deserialize_with = "parse_glob")]
    pub include: Vec<PathBuf>,
    #[serde(deserialize_with = "parse_glob", default = "Vec::new")]
    pub exclude: Vec<PathBuf>,
    pub tree: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RunConfig {
    pub prebuild: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AnalysisConfig {
    #[serde(default = "Vec::new")]
    pub compile_args: Vec<String>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            compile_args: Vec::new()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PresentationConfig {
    #[serde(
        deserialize_with = "parse_template",
        default = "default_class_template"
    )]
    pub class_template: String,
    
    #[serde(
        deserialize_with = "parse_template",
        default = "default_index_template"
    )]
    pub index_template: String,

    #[serde(
        deserialize_with = "parse_template",
        default = "default_head_template"
    )]
    pub head_template: String,

    #[serde(
        deserialize_with = "parse_template",
        default = "default_nav_template"
    )]
    pub nav_template: String,

    #[serde(
        deserialize_with = "parse_template",
        default = "default_file_template"
    )]
    pub file_template: String,

    #[serde(deserialize_with = "parse_template", default = "default_css")]
    pub css: String,

    #[serde(deserialize_with = "parse_template", default = "default_js")]
    pub js: String,
}

impl Default for PresentationConfig {
    fn default() -> Self {
        Self {
            class_template: default_class_template(),
            index_template: default_index_template(),
            head_template:  default_head_template(),
            file_template:  default_file_template(),
            nav_template:   default_nav_template(),
            css:            default_css(),
            js:             default_js(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub repository: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Options for the project
    pub project: ProjectConfig,
    /// Options for the documentation
    pub docs: DocsConfig,
    /// Options for the docs browser / navigation
    pub browser: BrowserConfig,
    #[serde(default)]
    /// Options for docs outlook
    pub presentation: PresentationConfig,
    /// Options for CMake
    pub cmake: Option<CMakeConfig>,
    /// Options for commands to run while building docs
    pub run: Option<RunConfig>,
    #[serde(default)]
    /// Options for LibClang
    pub analysis: AnalysisConfig,

    #[serde(skip)]
    pub input_dir: PathBuf,
    #[serde(skip)]
    pub output_dir: PathBuf,
}

impl Config {
    pub fn parse(input_dir: PathBuf, output_dir: PathBuf) -> Result<Config, String> {
        let mut config: Config = toml::from_str(
            &fs::read_to_string(input_dir.join("flash.toml"))
                .map_err(|e| format!("Unable to read flash.toml: {e}"))?,
        )
        .map_err(|e| format!("Unable to parse config: {e}"))?;

        config.input_dir = input_dir;
        config.output_dir = output_dir;
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

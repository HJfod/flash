use std::{fs, path::PathBuf};

use glob::glob;
use serde::{Deserialize, Deserializer};

fn def_class_template() -> String {
    include_str!("../templates/class.html").into()
}

fn def_link_template() -> String {
    include_str!("../templates/link.html").into()
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
pub struct PresentationConfig {
    #[serde(deserialize_with = "parse_template", default = "def_class_template")]
    pub class_template: String,
    #[serde(deserialize_with = "parse_template", default = "def_link_template")]
    pub link_template: String,
}

impl Default for PresentationConfig {
    fn default() -> Self {
        Self {
            class_template: def_class_template(),
            link_template: def_link_template(),
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
    #[serde(default)]
    /// Options for docs outlook
    pub presentation: PresentationConfig,
    /// Options for CMake
    pub cmake: Option<CMakeConfig>,
    /// Options for commands to run while building docs
    pub run: Option<RunConfig>,

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
        self.docs.include
            .iter()
            .filter(|p| !self.docs.exclude.contains(p))
            .collect()
    }
}

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
#[serde(rename_all = "lowercase")]
pub enum Mode {
    CMake,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::CMake
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub project: String,
    pub version: String,

    #[serde(deserialize_with = "parse_glob")]
    pub headers: Vec<PathBuf>,

    #[serde(default)]
    pub mode: Mode,
    pub cmake_config_args: Option<Vec<String>>,
    pub cmake_build_args: Option<Vec<String>>,
    #[serde(default = "cmake_build_default")]
    pub cmake_build: bool,
    pub cmake_infer_args_from: Option<PathBuf>,
    pub prebuild: Option<Vec<String>>,

    pub repository: Option<String>,
    pub tree: Option<String>,

    #[serde(deserialize_with = "parse_template", default = "def_class_template")]
    pub class_template: String,
    #[serde(deserialize_with = "parse_template", default = "def_link_template")]
    pub link_template: String,
}

impl Config {
    pub fn parse() -> Result<Config, String> {
        serde_json::from_str(
            &fs::read_to_string(std::env::current_dir().unwrap().join("flash.json"))
                .map_err(|e| format!("Unable to read flash.json: {e}"))?,
        )
        .map_err(|e| format!("Unable to parse config: {e}"))
    }
}

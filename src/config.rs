
use std::{path::PathBuf, fs};

use glob::glob;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    sources: Vec<PathBuf>,
    pub project: String,
    pub version: String,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub header_tree: Option<String>,
    pub source_tree: Option<String>,
    class_template: Option<PathBuf>,
}

impl Config {
    pub fn parse_file(input_dir: &PathBuf) -> Result<Config, String> {
        serde_json::from_str(
            &fs::read_to_string(input_dir.join("flash.json"))
            .map_err(|e| format!("Unable to read file: {}", e))?
        ).map_err(|e| format!("Unable to parse config: {}", e))
    }

    pub fn expanded_sources(&self) -> Vec<PathBuf> {
        self.sources.iter()
            .flat_map(|src| 
                glob(src.to_str().unwrap()).expect(
                    &format!("Invalid glob pattern {}", src.to_str().unwrap())
                ).map(|g| g.unwrap())
            )
            .collect()
    }

    pub fn class_template(&self) -> String {
        if let Some(ref user) = self.class_template {
            fs::read_to_string(user).expect("Unable to read class template")
        }
        else {
            include_str!("../templates/class.html").into()
        }
    }
}

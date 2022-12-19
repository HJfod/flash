use serde::Deserialize;
use std::{fs, path::PathBuf, process::Command};

use crate::config::Config;

#[derive(Deserialize, Clone)]
pub struct CompileCommand {
    pub directory: PathBuf,
    pub command: String,
    pub file: PathBuf,
}

impl CompileCommand {
    pub fn get_command_list(&self) -> Vec<String> {
        // Not using shlex because that screws up -DFMT_CONSTEVAL=\"\"
        self.command.split(" ")
            // Skip clang.exe
            .skip(1)
            // Only include parameters for defines, includes and std version 
            // because for some reason including all of them causes Clang 
            // to break
            .filter_map(|s|
                // Expand .rsp files into their include directives
                // For some reason LibClang just doesn't want to work with the .rsp 
                // files so got to do this
                if s.ends_with(".rsp") {
                    Some(
                        fs::read_to_string(
                            self.directory.join(s.replace("@", ""))
                        ).expect("Unable to read compiler .rsp includes file")
                            .split(" ")
                            .map(|s| s.to_owned())
                            .collect()
                    )
                } else {
                    if s.starts_with("-I") || s.starts_with("-D") || s.starts_with("-std") {
                        Some(vec![s.to_owned()])
                    }
                    else {
                        None
                    }
                }
            )
            .flatten()
            .collect()
    }
}

type CompileCommands = Vec<CompileCommand>;

pub fn cmake_configure(args: &Vec<String>) -> Result<(), String> {
    Command::new("cmake")
        .arg(".")
        .args(&["-B", "build"])
        .args(args)
        .spawn()
        .map_err(|e| format!("Error configuring CMake: {e}"))?
        .wait()
        .unwrap()
        .success()
        .then_some(())
        .ok_or("CMake configure failed".into())
}

pub fn cmake_build(args: &Vec<String>) -> Result<(), String> {
    Command::new("cmake")
        .args(["--build", "build"])
        .args(args)
        .spawn()
        .map_err(|e| format!("Error building CMake: {e}"))?
        .wait()
        .unwrap()
        .success()
        .then_some(())
        .ok_or("CMake build failed".into())
}

pub fn cmake_compile_commands() -> Result<CompileCommands, String> {
    serde_json::from_str(
        &fs::read_to_string(
            std::env::current_dir()
                .unwrap()
                .join("build")
                .join("compile_commands.json"),
        )
        .map_err(|e| format!("Unable to read compile_commands.json: {e}"))?,
    )
    .map_err(|e| format!("Unable to parse compile_commands.json: {e}"))
}

pub fn cmake_compile_args_for(config: &Config) -> Option<Vec<String>> {
    if let Some(ref from) = config.cmake_infer_args_from {
        for cmd in cmake_compile_commands().ok()? {
            if cmd.file.canonicalize().unwrap() == from.canonicalize().unwrap() {
                return Some(cmd.get_command_list());
            }
        }
    }
    None
}

use serde::Deserialize;
use std::{fs, path::PathBuf, process::Command, sync::Arc};

use crate::config::Config;

#[derive(Deserialize, Clone)]
pub struct CompileCommand {
    pub directory: PathBuf,
    pub command: String,
    pub file: PathBuf,
}

impl CompileCommand {
    pub fn get_command_list(&self, config: Arc<Config>) -> Vec<String> {
        // Not using shlex because that screws up -DFMT_CONSTEVAL=\"\"
        let mut list: Vec<String> = self.command.split(" ")
            // Skip clang.exe
            .skip(1)
            .flat_map(|s|
                // Expand .rsp files into their include directives
                // For some reason LibClang just doesn't want to work with the .rsp 
                // files so got to do this
                if s.ends_with(".rsp") {
                    fs::read_to_string(
                        self.directory.join(s.replace("@", ""))
                    ).expect("Unable to read compiler .rsp includes file")
                        .split(" ")
                        .map(|s| s.to_owned())
                        .collect()
                } else {
                    // Hacky fix to make sure -DMACRO="" defines MACRO as empty and not as ""
                    vec![s.to_owned().replace("=\"\"", "=")]
                }
            )
            // Add header root to include directories
            .chain(vec![format!("-I{}", config.input_dir.to_str().unwrap())])
            // Set working directory
            .chain(vec![format!("-working-directory={}", self.directory.to_str().unwrap())])
            // Add extra compile args
            .chain(config.analysis.compile_args.clone())
            .collect();

        // Passing -c crashes LibClang
        while let Some(ix) = list.iter().position(|s| s == "-c") {
            list.drain(ix..ix + 2);
        }

        list
    }
}

type CompileCommands = Vec<CompileCommand>;

pub fn cmake_configure(build_dir: &str, args: &Vec<String>) -> Result<(), String> {
    Command::new("cmake")
        .arg(".")
        .args(&["-B", build_dir])
        .args(args)
        .spawn()
        .map_err(|e| format!("Error configuring CMake: {e}"))?
        .wait()
        .unwrap()
        .success()
        .then_some(())
        .ok_or("CMake configure failed".into())
}

pub fn cmake_build(build_dir: &str, args: &Vec<String>) -> Result<(), String> {
    Command::new("cmake")
        .args(["--build", build_dir])
        .args(args)
        .spawn()
        .map_err(|e| format!("Error building CMake: {e}"))?
        .wait()
        .unwrap()
        .success()
        .then_some(())
        .ok_or("CMake build failed".into())
}

pub fn cmake_compile_commands(config: Arc<Config>) -> Result<CompileCommands, String> {
    serde_json::from_str(
        &fs::read_to_string(
            config
                .input_dir
                .join(&config.cmake.as_ref().unwrap().build_dir)
                .join("compile_commands.json"),
        )
        .map_err(|e| format!("Unable to read compile_commands.json: {e}"))?,
    )
    .map_err(|e| format!("Unable to parse compile_commands.json: {e}"))
}

pub fn cmake_compile_args_for(config: Arc<Config>) -> Option<Vec<String>> {
    let ref from = config.cmake.as_ref()?.infer_args_from;
    for cmd in cmake_compile_commands(config.clone()).ok()? {
        if cmd.file == config.input_dir.join(from) {
            return Some(cmd.get_command_list(config));
        }
    }
    None
}

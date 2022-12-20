use clap::Parser;
use config::Config;
use std::{fs, path::PathBuf, process::exit};

use crate::builder::build_docs_for;

mod builder;
mod cmake;
mod config;

#[derive(Parser, Debug)]
struct Args {
    /// Input directory with the flash.json file
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory where to place the generated docs
    #[arg(short, long)]
    output: PathBuf,

    /// Whether to overwrite output directory if it already exists
    #[arg(long, default_value_t = false)]
    overwrite: bool,
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    // Check output dir
    if args.output.exists() && !args.overwrite {
        println!(
            "Output directory {} already exists and no --overwrite option was specified, aborting",
            args.output.to_str().unwrap()
        );
        exit(1);
    }

    // Delete output dir if it exists
    if args.output.exists() {
        fs::remove_dir_all(&args.output).unwrap();
    }
    fs::create_dir_all(&args.output).unwrap();

    // Relink working directory to input dir and use absolute path for output
    // Not using fs::canonicalize because that returns UNC paths on Windows and
    // those break things
    let full_output = std::env::current_dir().unwrap().join(args.output);
    let full_input = std::env::current_dir().unwrap().join(args.input);
    std::env::set_current_dir(&full_input).expect(
        "Unable to set input dir as working directory \
            (probable reason is it doesn't exist)",
    );

    // Parse config
    let conf = Config::parse(full_input, full_output)?;

    // Build the docs
    println!("Building docs for {} ({})", conf.project, conf.version);
    build_docs_for(&conf)?;
    println!("Docs built for {}", conf.project);

    Ok(())
}

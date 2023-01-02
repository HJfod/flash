use crate::{analyze::create_docs, url::UrlPath};
use clap::Parser;
use config::Config;
use std::{fs, path::PathBuf, process::exit};

mod analyze;
mod builder;
mod cmake;
mod config;
mod html;
mod url;

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

#[tokio::main]
async fn main() -> Result<(), String> {
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

    let relative_output = if args.output.is_relative() {
        Some(UrlPath::try_from(&args.output).ok()).flatten()
    } else {
        None
    };

    // Relink working directory to input dir and use absolute path for output
    // Not using fs::canonicalize because that returns UNC paths on Windows and
    // those break things
    let full_output = if args.output.is_absolute() {
        args.output
    } else {
        std::env::current_dir().unwrap().join(args.output)
    };
    let full_input = if args.input.is_absolute() {
        args.input
    } else {
        std::env::current_dir().unwrap().join(args.input)
    };
    std::env::set_current_dir(&full_input).expect(
        "Unable to set input dir as working directory \
            (probable reason is it doesn't exist)",
    );

    // Parse config
    let conf = Config::parse(full_input, full_output, relative_output)?;

    // Build the docs
    println!(
        "Building docs for {} ({})",
        conf.project.name, conf.project.version
    );
    create_docs(conf.clone()).await?;
    println!("Docs built for {}", conf.project.name);

    Ok(())
}

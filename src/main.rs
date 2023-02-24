#![feature(let_chains)]
#![feature(is_some_and)]
#![feature(result_option_inspect)]
#![feature(iter_advance_by)]
#![feature(iter_intersperse)]

use crate::{analyze::create_docs, url::UrlPath, normalize::Normalize};
use clap::Parser;
use config::Config;
use std::{fs, path::{PathBuf, Path}, process::exit, io, time::Instant};

mod analyze;
mod builder;
mod cmake;
mod config;
mod html;
mod url;
mod normalize;
mod annotation;
mod lookahead;

#[derive(Parser, Debug)]
#[command(name("Flash"), version, about)]
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

fn remove_dir_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if entry.file_type()?.is_dir() {
            remove_dir_contents(&path)?;
            fs::remove_dir(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args = Args::parse();

    // Check if output dir exists
    if args.output.exists()
        // Check if it's empty
        && args.output.read_dir().map(|mut i| i.next().is_some()).unwrap_or(false)
        // Then overwrite must be specified
        && !args.overwrite
    {
        println!(
            "Output directory {} already exists and no --overwrite option was specified, aborting",
            args.output.to_str().unwrap()
        );
        exit(1);
    }

    // Clear output dir if it exists
    if args.output.exists() {
        remove_dir_contents(&args.output).unwrap();
    }
    else {
        fs::create_dir_all(&args.output).unwrap();
    }

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
        std::env::current_dir().unwrap().join(args.output).normalize()
    };
    let full_input = if args.input.is_absolute() {
        args.input
    } else {
        std::env::current_dir().unwrap().join(args.input).normalize()
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
    let now = Instant::now();
    create_docs(conf.clone()).await?;
    println!("Docs built for {} in {}s", conf.project.name, now.elapsed().as_secs());

    Ok(())
}

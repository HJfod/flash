
use std::{path::PathBuf, fs, process::exit};
use clap::Parser;
use config::Config;

use crate::builder::build_docs_for;

mod config;
mod builder;

#[derive(Parser, Debug)]
struct Args {
    /// Input directory with the flash.json file
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory where to place the generated docs
    #[arg(short, long)]
    output: PathBuf,

    #[arg(long, default_value_t = false)]
    overwrite: bool,
}

fn main() {
    let args = Args::parse();
    if args.output.exists() && !args.overwrite {
        println!(
            "Output directory {} already exists and no --overwrite option was specified, aborting",
            args.output.to_str().unwrap()
        );
        exit(1);
    }
    fs::remove_dir_all(&args.output).unwrap();
    fs::create_dir_all(&args.output).unwrap();
    let conf = Config::parse_file(&args.input).unwrap();
    println!("Building docs for {} ({})", conf.project, conf.version);
    build_docs_for(&conf, &args.input, &args.output);
    println!("Docs built for {}", conf.project);
}

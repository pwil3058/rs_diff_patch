use clap::Parser;
use std::fs::File;
use std::io;
use std::path::PathBuf;

use log;
use serde::{Deserialize, Serialize};
use serde_json;
use stderrlog;

use diff_lib::{DiffChunk, Lines, Modifications};

#[derive(Debug, Default, Serialize, Deserialize)]
struct Diff {
    before_path: PathBuf,
    after_path: PathBuf,
    chunks: Vec<DiffChunk>,
}

impl Diff {
    fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(required = true, short, long)]
    context: usize,
    #[arg(required = true)]
    before_file_path: PathBuf,
    #[arg(required = true)]
    after_file_path: PathBuf,
}

fn main() {
    let args = Cli::parse();

    stderrlog::new().module(module_path!()).init().unwrap();

    let before_file = match File::open(&args.before_file_path) {
        Ok(file) => file,
        Err(err) => {
            log::error!("Error opening {:?}: {err}", args.before_file_path);
            std::process::exit(1);
        }
    };
    let before_lines = match Lines::read(before_file) {
        Ok(lines) => lines,
        Err(err) => {
            log::error!("Error reading {:?}: {err}", args.before_file_path);
            std::process::exit(2);
        }
    };

    let after_file = match File::open(&args.after_file_path) {
        Ok(file) => file,
        Err(err) => {
            log::error!("Error opening {:?}: {err}", args.after_file_path);
            std::process::exit(3);
        }
    };
    let after_lines = match Lines::read(after_file) {
        Ok(lines) => lines,
        Err(err) => {
            log::error!("Error reading {:?}: {err}", args.after_file_path);
            std::process::exit(4);
        }
    };

    let modifications = Modifications::new(before_lines, after_lines);
    let diff = Diff {
        before_path: args.before_file_path,
        after_path: args.after_file_path,
        chunks: modifications.chunks::<DiffChunk>(args.context).collect(),
    };

    match diff.to_writer(&mut io::stdout()) {
        Ok(_) => (),
        Err(err) => {
            log::error!("Error writing diff: {err}");
            std::process::exit(5);
        }
    }
}

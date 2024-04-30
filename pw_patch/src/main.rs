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
    pub fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }

    pub fn from_reader<R: io::Read>(reader: &mut R) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(reader)
    }
}

#[derive(Parser)]
struct Cli {
    #[arg(required = true)]
    patch_path: PathBuf,
}

fn main() {
    let args = Cli::parse();

    stderrlog::new().module(module_path!()).init().unwrap();

    let mut patch_file = match File::open(&args.patch_path) {
        Ok(file) => file,
        Err(err) => {
            log::error!("Error opening {:?}: {err}", args.patch_path);
            std::process::exit(1);
        }
    };

    let _diff: Diff = match Diff::from_reader(&mut patch_file) {
        Ok(diff) => diff,
        Err(err) => {
            log::error!("Error reading patch file: {err}");
            std::process::exit(1)
        }
    };
}

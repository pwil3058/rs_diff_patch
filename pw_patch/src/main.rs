use clap::Parser;
use std::fs::File;
use std::path::PathBuf;

use log;
use stderrlog;

use diff_lib::Diff;

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

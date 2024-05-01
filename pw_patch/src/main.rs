use clap::Parser;
use std::fs::File;
use std::path::PathBuf;

use log;
use stderrlog;

use diff_lib::apply::ApplyChunks;
use diff_lib::{Diff, Lines};

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long)]
    reverse: bool,
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

    let diff: Diff = match Diff::from_reader(&mut patch_file) {
        Ok(diff) => diff,
        Err(err) => {
            log::error!("Error reading patch file: {err}");
            std::process::exit(1)
        }
    };

    let patchable_path = diff.before_path();
    let patchable_file = match File::open(patchable_path) {
        Ok(file) => file,
        Err(err) => {
            log::error!("Error opening {patchable_path:?}: {err}");
            std::process::exit(1);
        }
    };
    let patchable_lines = match Lines::read(patchable_file) {
        Ok(lines) => lines,
        Err(err) => {
            log::error!("Error reading {patchable_path:?}: {err}");
            std::process::exit(1);
        }
    };

    match temp_file::TempFile::in_dir(".") {
        Ok(temp_file) => {
            let mut writer = match File::create(temp_file.path()) {
                Ok(file) => file,
                Err(err) => {
                    log::error!("Error opening temporary file: {err}");
                    std::process::exit(1);
                }
            };
            match diff.apply_into(&patchable_lines, &mut writer, args.reverse) {
                Ok(stats) => {
                    match std::fs::rename(temp_file.path(), patchable_path) {
                        Ok(_) => log::info!("{stats:?}"),
                        Err(err) => {
                            log::error!("Error writing patched file: {err}");
                            std::process::exit(1);
                        }
                    };
                }
                Err(err) => {
                    log::error!("Patch failed to apply: {err}");
                    std::process::exit(1);
                }
            }
        }
        Err(err) => {
            log::error!("Error creating temp file: {err}");
            std::process::exit(1)
        }
    }
}

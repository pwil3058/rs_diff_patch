// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use clap::Parser;
use json_diff_lib::PatchSet;
use json_diff_lib::apply::DirPatchApplier;
use std::fs::File;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "jpatch",
    version = "1.0",
    author = "Peter Williams",
    about = "Applies human-reviewable JSON patch files to a target directory workspace."
)]
struct PatchCli {
    /// Target root directory where the patch actions will execute
    #[arg(short, long, default_value = ".")]
    target_dir: PathBuf,

    /// Path to the JSON patch file (defaults to stdin if omitted)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Rollback changes by applying the patch file in reverse mode
    #[arg(short, long)]
    reverse: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = PatchCli::parse();

    // 1. Ingest the patch set from a file handle or standard input stream
    let patch_set = match cli.input {
        Some(path) => {
            let file = File::open(path)?;
            PatchSet::from_reader(file)?
        }
        None => {
            let stdin = std::io::stdin();
            PatchSet::from_reader(stdin.lock())?
        }
    };

    // 2. Execute the operations transactionally onto the destination path root via your library core
    DirPatchApplier::apply_patch_set(&cli.target_dir, &patch_set, cli.reverse)?;

    log::info!("Patch sequence executed successfully!");
    Ok(())
}

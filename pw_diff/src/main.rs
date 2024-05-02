use clap::Parser;
use std::io;
use std::path::PathBuf;

use log;
use stderrlog;

use diff_lib::ChangeDiff;

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

    let diff = match ChangeDiff::new(&args.before_file_path, &args.after_file_path, args.context) {
        Ok(diff) => diff,
        Err(err) => {
            log::error!("Error: {err}");
            std::process::exit(1);
        }
    };

    match diff.to_writer(&mut io::stdout()) {
        Ok(_) => (),
        Err(err) => {
            log::error!("Error writing diff: {err}");
            std::process::exit(5);
        }
    }
}

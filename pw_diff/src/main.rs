use clap::Parser;
use std::io;
use std::path::PathBuf;

use log;
use stderrlog;
use stderrlog::LogLevelNum;

use diff_lib::Diff;

#[derive(Parser)]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, help = "Control reporting")]
    verbose: u8,
    #[arg(
        short,
        long,
        help = "Number of lines of context to use",
        default_value = "2"
    )]
    context: usize,
    #[arg(required = true)]
    before_file_path: PathBuf,
    #[arg(required = true)]
    after_file_path: PathBuf,
}

fn main() {
    let args = Cli::parse();

    stderrlog::new()
        .module(module_path!())
        .verbosity(LogLevelNum::from(args.verbose as usize))
        .init()
        .unwrap();

    let diff = match Diff::new(&args.before_file_path, &args.after_file_path, args.context) {
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

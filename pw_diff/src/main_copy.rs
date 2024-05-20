// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::io;
use std::path::PathBuf;

use clap::Parser;
use log;
use stderrlog;
use stderrlog::LogLevelNum;

use pw_diff_lib::diff::Diff;

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
    context: u8,
    #[arg(required = true)]
    before_file_path: PathBuf,
    #[arg(required = true)]
    after_file_path: PathBuf,
}

fn _main_copy() {
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

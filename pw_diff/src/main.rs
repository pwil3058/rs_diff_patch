use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[arg(required = true)]
    before_file: PathBuf,
    #[arg(required = true)]
    after_file: PathBuf,
}

fn main() {
    let args = Cli::parse();
    println!("Before File: {:?}", args.before_file);
    println!("After File: {:?}", args.after_file);
}

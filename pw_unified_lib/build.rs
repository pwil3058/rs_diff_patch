// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::fs;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/unified_parser.laps");
    //fs::remove_file("src/unified_parser.rs");
    match Command::new("lap_gen")
        .args([
            "-o",
            "src/unified_parser.rs",
            "-f",
            "src/unified_parser.laps",
        ])
        .status()
    {
        Ok(status) => {
            if status.success() {
                Command::new("rustfmt")
                    .args(["src/unified_parser.rs"])
                    .status()
                    .unwrap();
            } else {
                eprintln!("failed prebuild formatting: {status}");
            };
        }
        Err(err) => eprintln!("failed lap_gen: {err}: check specification"),
    }
}

// Copyright 2021 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/unified.laps");
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
                panic!("failed prebuild: {status}");
            };
        }
        Err(err) => panic!("Build error: {err}"),
    }
    // println!("cargo:rerun-if-changed=buildx");
}

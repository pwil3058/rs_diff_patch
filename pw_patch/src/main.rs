// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::fs;
use std::fs::File;
use std::path::PathBuf;

use clap::Parser;
use log;
use stderrlog;
use stderrlog::LogLevelNum;

use pw_diff_lib::{
    apply_bytes::ApplyChunksClean, apply_text::ApplyChunksFuzzy, diff::Diff, sequence::Seq,
};

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, help = "Control reporting")]
    verbose: u8,
    #[arg(short, long, help = "Apply the patch in reverse")]
    reverse: bool,
    #[arg(required = true)]
    patch_path: PathBuf,
}

fn main() {
    let args = Cli::parse();

    stderrlog::new()
        .module(module_path!())
        .verbosity(LogLevelNum::from(args.verbose as usize))
        .init()
        .unwrap();

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

    match diff {
        Diff::TextChange(diff) => {
            let patchable_path = diff.before_path();
            let patchable_file = match File::open(patchable_path) {
                Ok(file) => file,
                Err(err) => {
                    log::error!("Error opening {patchable_path:?}: {err}");
                    std::process::exit(1);
                }
            };
            let patchable_lines = match Seq::<String>::read(patchable_file) {
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
        Diff::TextAdd(path_and_lines) => {
            let path = path_and_lines.path();
            if args.reverse {
                match fs::remove_file(path) {
                    Ok(_) => log::info!("{path:?} deleted"),
                    Err(err) => {
                        log::error!("{path:?} deletion failed: {err}");
                        std::process::exit(1)
                    }
                }
            } else {
                match File::create_new(path) {
                    Ok(mut file) => match path_and_lines.write_into(&mut file) {
                        Ok(_) => log::info!("{path:?} created"),
                        Err(err) => {
                            log::error!("{path:?} creation failed: {err}")
                        }
                    },
                    Err(err) => {
                        log::error!("{path:?} creation failed: {err}")
                    }
                }
            }
        }
        Diff::TextRemove(path_and_lines) => {
            let path = path_and_lines.path();
            if args.reverse {
                match File::create_new(path) {
                    Ok(mut file) => match path_and_lines.write_into(&mut file) {
                        Ok(_) => log::info!("{path:?} created"),
                        Err(err) => {
                            log::error!("{path:?} creation failed: {err}")
                        }
                    },
                    Err(err) => {
                        log::error!("{path:?} creation failed: {err}")
                    }
                }
            } else {
                match fs::remove_file(path) {
                    Ok(_) => log::info!("{path:?} deleted"),
                    Err(err) => {
                        log::error!("{path:?} deletion failed: {err}");
                        std::process::exit(1)
                    }
                }
            }
        }
        Diff::ByteChange(diff) => {
            let patchable_path = diff.before_path();
            let patchable_file = match File::open(patchable_path) {
                Ok(file) => file,
                Err(err) => {
                    log::error!("Error opening {patchable_path:?}: {err}");
                    std::process::exit(1);
                }
            };
            let patchable_bytes = match Seq::<u8>::read(patchable_file) {
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
                    match diff.apply_into(&patchable_bytes, &mut writer, args.reverse) {
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
        Diff::ByteAdd(path_and_bytes) => {
            let path = path_and_bytes.path();
            if args.reverse {
                match fs::remove_file(path) {
                    Ok(_) => log::info!("{path:?} deleted"),
                    Err(err) => {
                        log::error!("{path:?} deletion failed: {err}");
                        std::process::exit(1)
                    }
                }
            } else {
                match File::create_new(path) {
                    Ok(mut file) => match path_and_bytes.write_into(&mut file) {
                        Ok(_) => log::info!("{path:?} created"),
                        Err(err) => {
                            log::error!("{path:?} creation failed: {err}")
                        }
                    },
                    Err(err) => {
                        log::error!("{path:?} creation failed: {err}")
                    }
                }
            }
        }
        Diff::ByteRemove(path_and_bytes) => {
            let path = path_and_bytes.path();
            if args.reverse {
                match File::create_new(path) {
                    Ok(mut file) => match path_and_bytes.write_into(&mut file) {
                        Ok(_) => log::info!("{path:?} created"),
                        Err(err) => {
                            log::error!("{path:?} creation failed: {err}")
                        }
                    },
                    Err(err) => {
                        log::error!("{path:?} creation failed: {err}")
                    }
                }
            } else {
                match fs::remove_file(path) {
                    Ok(_) => log::info!("{path:?} deleted"),
                    Err(err) => {
                        log::error!("{path:?} deletion failed: {err}");
                        std::process::exit(1)
                    }
                }
            }
        }
    }
}

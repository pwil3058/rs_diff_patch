// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use clap::Parser;
use json_diff_lib::JsonDiff;
use json_diff_lib::PatchSet;
use json_diff_lib::dir_diff_scanner::DirDiffScanner;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "jdiff",
    version = "1.0",
    author = "Peter Williams",
    about = "Smart JSON differential engine inferring actions from file parameters."
)]
struct Cli {
    /// Zero, one, or two target file system paths to evaluate
    #[arg(value_name = "PATHS")]
    paths: Vec<PathBuf>,

    /// Git revision specifier handles (e.g., short hash, branch, or HEAD~1). Pass up to two times.
    #[arg(short, long = "revision", value_name = "REV")]
    revisions: Vec<String>,

    /// Number of context lines to anchor modifications
    #[arg(short, long, default_value_t = 3)]
    context: u8,

    /// Paths or folder names to exclude from directory scans (can be passed multiple times)
    #[arg(short, long)]
    exclude: Vec<String>,

    /// Custom title metadata attribute for the patch envelope header
    #[arg(long)]
    title: Option<String>,

    /// Custom description summary text for the patch envelope header
    #[arg(long)]
    description: Option<String>,

    /// Override the author profile metadata (defaults to local git config identity)
    #[arg(long)]
    author: Option<String>,

    /// Output path for the generated patch file (defaults to stdout if omitted)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Prevent JSON pretty-printing to compress the output file footprint size
    #[arg(long)]
    no_pretty: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = Cli::parse();

    // Enforce parameter guard assertions before starting execution
    if cli.paths.len() > 2 {
        eprintln!("Error: Too many file paths supplied. Expected 0, 1, or 2 paths.");
        std::process::exit(1);
    }
    if cli.revisions.len() > 2 {
        eprintln!("Error: Too many revision flags supplied. Expected maximum of 2.");
        std::process::exit(1);
    }
    if cli.paths.len() == 2 && !cli.revisions.is_empty() {
        eprintln!(
            "Error: Cannot combine two distinct filesystem paths with git revision parameters."
        );
        std::process::exit(1);
    }

    let mut patch_set = PatchSet::default();
    let context = cli.context;

    // Core inference engine loop
    match (cli.paths.len(), cli.revisions.len()) {
        // =========================================================================
        // CASE 1: 0 Paths, 0 Revisions -> Complete Git Workspace Scan
        // =========================================================================
        (0, 0) => {
            let pwd = std::env::current_dir()?;
            verify_in_git_workspace(&pwd)?;
            patch_set = DirDiffScanner::compare_git_workspace(&pwd, context, &cli.exclude)?;
        }

        // =========================================================================
        // CASE 2: 1 Path, 0 Revisions -> Specific Git Target Scan (File or Folder vs HEAD)
        // =========================================================================
        (1, 0) => {
            let target = &cli.paths[0];
            verify_in_git_workspace(target)?;

            if target.is_dir() {
                patch_set = DirDiffScanner::compare_git_workspace(target, context, &cli.exclude)?;
            } else {
                if let Some(diff) = JsonDiff::generate_from_git(target, context)? {
                    patch_set.diffs.push(diff);
                }
            }
        }

        // =========================================================================
        // CASE 3: 1 Path, 1 Revision -> Target vs Specific Historical Commit Snapshot
        // =========================================================================
        (1, 1) => {
            let target = &cli.paths[0];
            verify_in_git_workspace(target)?;
            let rev = &cli.revisions[0];

            if target.is_dir() {
                patch_set = DirDiffScanner::compare_git_commits(target, rev, "HEAD", context)?;
            } else {
                if let Some(diff) =
                    JsonDiff::generate_from_git_commits(target, rev, "HEAD", context)?
                {
                    patch_set.diffs.push(diff);
                }
            }
        }

        // =========================================================================
        // CASE 4: 1 Path, 2 Revisions -> Diff a file between two distinct histories
        // =========================================================================
        (1, 2) => {
            let target = &cli.paths[0];
            verify_in_git_workspace(target)?;
            if target.is_dir() {
                patch_set = DirDiffScanner::compare_git_commits(
                    target,
                    &cli.revisions[0],
                    &cli.revisions[1],
                    context,
                )?;
            } else {
                if let Some(diff) = JsonDiff::generate_from_git_commits(
                    target,
                    &cli.revisions[0],
                    &cli.revisions[1],
                    context,
                )? {
                    patch_set.diffs.push(diff);
                }
            }
        }

        // =========================================================================
        // CASE 6: 0 Paths, 2 Revisions -> Complete Directory Tree Diff Between Commits
        // =========================================================================
        (0, 2) => {
            let pwd = std::env::current_dir()?;
            verify_in_git_workspace(&pwd)?;

            // Runs your test-backed git commit tree scanner natively over the full project!
            patch_set = DirDiffScanner::compare_git_commits(
                &pwd,
                &cli.revisions[0],
                &cli.revisions[1],
                context,
            )?;
        }

        // =========================================================================
        // CASE 5: 2 Paths, 0 Revisions -> Standard Plain Filesystem Diff (Files or Folders)
        // =========================================================================
        (2, 0) => {
            let before = &cli.paths[0];
            let after = &cli.paths[1];

            if before.is_dir() && after.is_dir() {
                patch_set =
                    DirDiffScanner::compare_directories(before, after, context, &cli.exclude)?;
            } else if before.is_file() && after.is_file() {
                if let Some(diff) = JsonDiff::new(
                    before.parent().unwrap(),
                    after.parent().unwrap(),
                    Path::new(before.file_name().unwrap()),
                    context,
                )? {
                    patch_set.diffs.push(diff);
                }
            } else {
                eprintln!(
                    "Error: Path type mismatch. Both parameters must be files or both must be directories."
                );
                std::process::exit(1);
            }
        }

        _ => {
            eprintln!("Error: Unrecognized parameter argument permutation combination.");
            std::process::exit(1);
        }
    }

    // 4. Enrich metadata headers and fallback to global git profile configurations
    patch_set.title = cli
        .title
        .or(Some(String::from("Smart Environment Delta Patch")));
    patch_set.description = cli.description;

    let author_identity = match cli.author {
        Some(explicit) => explicit,
        None => {
            if let Ok(git_config) = git2::Config::open_default() {
                let name = git_config.get_string("user.name").unwrap_or_default();
                let email = git_config.get_string("user.email").unwrap_or_default();
                if !name.is_empty() && !email.is_empty() {
                    format!("{} <{}>", name, email)
                } else {
                    String::from("System Automated jdiff User")
                }
            } else {
                String::from("System Automated jdiff User")
            }
        }
    };
    patch_set
        .metadata
        .insert(String::from("author"), author_identity);
    patch_set.metadata.insert(
        String::from("generated_at"),
        chrono::Local::now().to_rfc3339(),
    );

    // 5. Output file serialization pipeline
    let pretty = !cli.no_pretty;
    match cli.output {
        Some(path) => {
            let file = File::create(path)?;
            patch_set.to_writer(file, pretty)?;
        }
        None => {
            let stdout = std::io::stdout();
            patch_set.to_writer(stdout.lock(), pretty)?;
        }
    }

    Ok(())
}

/// Verification helper to crash fast if a git execution is attempted in an ordinary directory
fn verify_in_git_workspace(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if git2::Repository::discover(path).is_err() {
        eprintln!(
            "Error: Target path '{:?}' is not inside a tracking Git repository workspace.",
            path
        );
        std::process::exit(1);
    }
    Ok(())
}

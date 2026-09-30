// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use clap::{Parser, Subcommand};
use json_diff_lib::dir_diff_scanner::DirDiffScanner;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "jdiff",
    version = "0.1.0",
    author = "Peter Williams",
    about = "Generates human-reviewable JSON patch sets for directories and Git repositories."
)]
struct DiffCli {
    /// Number of context lines to anchor modifications
    #[arg(short, long, default_value_t = 3)]
    context: u8,

    /// Paths or folder names to exclude from directory scans (can be passed multiple times)
    #[arg(short, long)]
    exclude: Vec<String>, // 👈 New vector array flag

    /// Output path for the generated patch file (defaults to stdout if omitted)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Prevent JSON pretty-printing to compress the output file footprint size
    #[arg(long)]
    no_pretty: bool,

    #[command(subcommand)]
    command: DiffCommand,
}

#[derive(Subcommand, Debug)]
enum DiffCommand {
    /// Compares two standard filesystem directories
    Dir { before: PathBuf, after: PathBuf },
    /// Compares a live Git repository workspace against its current HEAD baseline
    GitWorkspace {
        #[arg(default_value = ".")]
        repo_path: PathBuf,
    },
    /// Compares two specific historical Git commit states inside a repository
    GitCommits {
        before_commit: String,
        after_commit: String,
        #[arg(default_value = ".")]
        repo_path: PathBuf,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = DiffCli::parse();

    let raw_target_path = match &cli.command {
        DiffCommand::Dir { before, .. } => before.clone(),
        DiffCommand::GitWorkspace { repo_path } => repo_path.clone(),
        DiffCommand::GitCommits { repo_path, .. } => repo_path.clone(),
    };

    let target_path = std::fs::canonicalize(&raw_target_path).unwrap_or_else(|_| raw_target_path);

    let mut unified_excludes = load_ignore_patterns(&target_path);
    unified_excludes.extend(cli.exclude.clone());

    // 1. Generate the patch set via the targeted scanning library branch
    let patch_set = match cli.command {
        DiffCommand::Dir { before, after } => {
            DirDiffScanner::compare_directories(&before, &after, cli.context, &unified_excludes)?
        }
        DiffCommand::GitWorkspace { repo_path } => {
            DirDiffScanner::compare_git_workspace(&repo_path, cli.context, &unified_excludes)?
        }
        DiffCommand::GitCommits {
            before_commit,
            after_commit,
            repo_path,
        } => DirDiffScanner::compare_git_commits(
            &repo_path,
            &before_commit,
            &after_commit,
            cli.context,
        )?,
    };

    // 2. Stream the serialized patch document to the appropriate destination handle
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

/// Collects all exclusion rules from a local `.jdiffignore` file, or falls
/// back to a global home configuration profile if found.
fn load_ignore_patterns(repo_path: &Path) -> Vec<String> {
    let mut patterns = Vec::new();

    // 1. Check for a local project-level ignore file first
    let local_ignore = repo_path.join(".jdiffignore");
    if local_ignore.exists() {
        if let Ok(local_patterns) = parse_ignore_file(&local_ignore) {
            patterns.extend(local_patterns);
            return patterns; // Local overrides global configs completely
        }
    }

    // 2. Fall back to a global user profile configuration (~/.config/jdiff/ignore)
    if let Some(mut global_config) = dirs::home_dir() {
        global_config.push(".config");
        global_config.push("jdiff");
        global_config.push("ignore");

        if global_config.exists() {
            if let Ok(global_patterns) = parse_ignore_file(&global_config) {
                patterns.extend(global_patterns);
            }
        }
    }

    patterns
}

/// Helper to parse an ignore file line-by-line, stripping out comments and spaces
fn parse_ignore_file(path: &Path) -> std::io::Result<Vec<String>> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut patterns = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let trimmed = line.trim();

        // Skip completely empty lines or comment tracking blocks safely
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        patterns.push(trimmed.to_string());
    }

    Ok(patterns)
}

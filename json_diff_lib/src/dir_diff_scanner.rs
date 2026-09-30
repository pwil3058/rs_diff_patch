// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use crate::git_helper::collect_files_from_commit;
use crate::{JsonDiff, PatchSet};
use git2::Repository;
use path_utilities::UsefulPathMethods;
use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

pub struct DirDiffScanner;

impl DirDiffScanner {
    /// Evaluates two directory branches side-by-side using an O(N) linear sweep.
    pub fn compare_directories(
        before_dir: impl AsRef<Path>,
        after_dir: impl AsRef<Path>,
        context: u8,
        excludes: &[String],
    ) -> io::Result<PatchSet> {
        let before_dir = before_dir.as_ref();
        let after_dir = after_dir.as_ref();
        let mut patch_set = PatchSet::default();

        let before_files = before_dir.collect_relative_files(excludes)?;
        let after_files = after_dir.collect_relative_files(excludes)?;

        let mut before_iter = before_files.iter().peekable();
        let mut after_iter = after_files.iter().peekable();

        loop {
            match (before_iter.peek(), after_iter.peek()) {
                (Some(&b_path), Some(&a_path)) => {
                    if b_path == a_path {
                        let diff = JsonDiff::new(before_dir, after_dir, b_path, context)?;
                        if !diff.is_empty() {
                            patch_set.diffs.push(diff);
                        }
                        before_iter.next();
                        after_iter.next();
                    } else if b_path < a_path {
                        let diff = JsonDiff::new(before_dir, after_dir, b_path, context)?;
                        patch_set.diffs.push(diff);
                        before_iter.next();
                    } else {
                        let diff = JsonDiff::new(before_dir, after_dir, a_path, context)?;
                        patch_set.diffs.push(diff);
                        after_iter.next();
                    }
                }
                (Some(&b_path), None) => {
                    let diff = JsonDiff::new(before_dir, after_dir, b_path, context)?;
                    patch_set.diffs.push(diff);
                    before_iter.next();
                }
                (None, Some(&a_path)) => {
                    let diff = JsonDiff::new(before_dir, after_dir, a_path, context)?;
                    patch_set.diffs.push(diff);
                    after_iter.next();
                }
                (None, None) => break,
            }
        }

        Ok(patch_set)
    }

    /// Compares an entire repository workspace between two historical commit points,
    /// generating a unified, human-readable `PatchSet`.
    pub fn compare_git_commits<P: AsRef<Path>>(
        repo_path: P,
        before_commit_id: &str,
        after_commit_id: &str,
        context: u8,
    ) -> io::Result<PatchSet> {
        let mut patch_set = PatchSet::default();

        // 1. Discover and bind to the target repository context
        let repo = Repository::discover(repo_path.as_ref()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Git repository not found: {}", e),
            )
        })?;

        // 2. Fetch the sorted relative file trees from both commit snapshots
        let before_files = collect_files_from_commit(&repo, before_commit_id)?;
        let after_files = collect_files_from_commit(&repo, after_commit_id)?;

        let mut before_iter = before_files.iter().peekable();
        let mut after_iter = after_files.iter().peekable();

        let workdir = repo.workdir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Bare repositories cannot resolve relative target paths",
            )
        })?;

        // 3. High-speed linear two-pointer matching loop
        loop {
            match (before_iter.peek(), after_iter.peek()) {
                (Some(&b_path), Some(&a_path)) => {
                    if b_path == a_path {
                        // The file exists in both commits. Generate an in-memory dual-commit diff.
                        let full_workdir_path = workdir.join(b_path);
                        let diff = JsonDiff::generate_from_git_commits(
                            &full_workdir_path,
                            before_commit_id,
                            after_commit_id,
                            context,
                        )?;

                        if !diff.is_empty() {
                            patch_set.diffs.push(diff);
                        }
                        before_iter.next();
                        after_iter.next();
                    } else if b_path < a_path {
                        // File was present before but missing after -> Deleted file
                        let full_workdir_path = workdir.join(b_path);
                        let diff =
                            JsonDiff::new(&full_workdir_path, Path::new(""), b_path, context)?;
                        patch_set.diffs.push(diff);
                        before_iter.next();
                    } else {
                        // File is missing before but present after -> Added file
                        let full_workdir_path = workdir.join(a_path);
                        let diff =
                            JsonDiff::new(Path::new(""), &full_workdir_path, a_path, context)?;
                        patch_set.diffs.push(diff);
                        after_iter.next();
                    }
                }
                (Some(&b_path), None) => {
                    let full_workdir_path = workdir.join(b_path);
                    let diff = JsonDiff::new(&full_workdir_path, Path::new(""), b_path, context)?;
                    patch_set.diffs.push(diff);
                    before_iter.next();
                }
                (None, Some(&a_path)) => {
                    let full_workdir_path = workdir.join(a_path);
                    let diff = JsonDiff::new(Path::new(""), &full_workdir_path, a_path, context)?;
                    patch_set.diffs.push(diff);
                    after_iter.next();
                }
                (None, None) => break,
            }
        }

        Ok(patch_set)
    }

    /// Scans a live Git repository working directory, cleanly isolating root bounds.
    pub fn compare_git_workspace<P: AsRef<Path>>(
        repo_path: P,
        context: u8,
        user_excludes: &[String],
    ) -> io::Result<PatchSet> {
        let root = repo_path.as_ref();
        let mut patch_set = PatchSet::default();

        let repo = Repository::discover(root).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Git repository not found: {e}"),
            )
        })?;

        let workdir = repo.workdir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Bare repositories cannot resolve relative target paths",
            )
        })?;

        // 1. Compile your unified exclusion list
        let mut local_excludes = vec![String::from(".git")];
        local_excludes.extend(user_excludes.iter().cloned());

        // 2. Fetch the files lists
        let raw_before_files =
            crate::git_helper::collect_files_from_commit(&repo, "HEAD").unwrap_or_default();
        let before_files: BTreeSet<PathBuf> = raw_before_files
            .into_iter()
            .filter(|p| {
                let s = p.to_string_lossy();
                !local_excludes.iter().any(|pattern| s.contains(pattern))
            })
            .collect();

        let after_files = root.collect_relative_files(&local_excludes)?;

        let mut before_iter = before_files.iter().peekable();
        let mut after_iter = after_files.iter().peekable();

        // 3. High-speed linear matching loop
        loop {
            match (before_iter.peek(), after_iter.peek()) {
                (Some(&b_path), Some(&a_path)) => {
                    if b_path == a_path {
                        let full_path = workdir.join(b_path);
                        let diff = JsonDiff::generate_from_git(&full_path, context)?;
                        if !diff.is_empty() {
                            patch_set.diffs.push(diff);
                        }
                        before_iter.next();
                        after_iter.next();
                    } else if b_path < a_path {
                        // FIX: Pass the base workspace directory root and empty path directly!
                        // This allows JsonDiff::new to compute absolute joins accurately.
                        let diff = JsonDiff::new(workdir, Path::new(""), b_path, context)?;
                        patch_set.diffs.push(diff);
                        before_iter.next();
                    } else {
                        // FIX: Pass empty path and base workspace directory root directly!
                        let diff = JsonDiff::new(Path::new(""), workdir, a_path, context)?;
                        patch_set.diffs.push(diff);
                        after_iter.next();
                    }
                }
                (Some(&b_path), None) => {
                    let diff = JsonDiff::new(workdir, Path::new(""), b_path, context)?;
                    patch_set.diffs.push(diff);
                    before_iter.next();
                }
                (None, Some(&a_path)) => {
                    let diff = JsonDiff::new(Path::new(""), workdir, a_path, context)?;
                    patch_set.diffs.push(diff);
                    after_iter.next();
                }
                (None, None) => break,
            }
        }

        Ok(patch_set)
    }

    /// Scans a live Git repository working directory, comparing all active local
    /// checked-out files against their current Git baseline index records.
    pub fn compare_git_workspace_bad<P: AsRef<Path>>(
        repo_path: P,
        context: u8,
        user_excludes: &[String],
    ) -> io::Result<PatchSet> {
        let root = repo_path.as_ref();
        // Dynamic expansion array injection:
        // Automatically combines the mandatory downstream ".git" pattern with the user flags
        let mut excludes = vec![String::from(".git")];
        excludes.extend(user_excludes.iter().cloned());

        let mut patch_set = PatchSet::default();

        // 1. Bind to the local Git repository context
        let repo = Repository::discover(root).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Git repository not found: {e}"),
            )
        })?;

        let workdir = repo.workdir().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "Bare repositories cannot resolve relative target paths",
            )
        })?;

        // 2. Gather files from both locations side-by-side:
        // 'before' comes from Git's tracked index, 'after' comes from your new disk utility
        let before_files =
            crate::git_helper::collect_files_from_commit(&repo, "HEAD").unwrap_or_default(); // Fallback to empty if repository has no commits yet
        let after_files = root.collect_relative_files(user_excludes)?; // Uses your new path_utilities method!

        let mut before_iter = before_files.iter().peekable();
        let mut after_iter = after_files.iter().peekable();

        // 3. High-speed linear two-pointer loop matching disk files against Git tracking paths
        loop {
            match (before_iter.peek(), after_iter.peek()) {
                (Some(&b_path), Some(&a_path)) => {
                    if b_path == a_path {
                        let full_path = workdir.join(b_path);

                        // Leverage your working single-file git extractor to fetch index vs disk data
                        let diff = JsonDiff::generate_from_git(&full_path, context)?;

                        if !diff.is_empty() {
                            patch_set.diffs.push(diff);
                        }
                        before_iter.next();
                        after_iter.next();
                    } else if b_path < a_path {
                        // File was tracked in Git but is completely missing from disk -> Deleted
                        let full_before_path = workdir.join(b_path);
                        let diff =
                            JsonDiff::new(&full_before_path, Path::new(""), b_path, context)?;
                        patch_set.diffs.push(diff);
                        before_iter.next();
                    } else {
                        // File is sitting on disk but doesn't exist in Git yet -> Untracked Addition
                        let full_after_path = workdir.join(a_path);
                        let diff = JsonDiff::new(Path::new(""), &full_after_path, a_path, context)?;
                        patch_set.diffs.push(diff);
                        after_iter.next();
                    }
                }
                (Some(&b_path), None) => {
                    // Remaining tracked Git items are deleted from disk
                    let full_before_path = workdir.join(b_path);
                    let diff = JsonDiff::new(&full_before_path, Path::new(""), b_path, context)?;
                    patch_set.diffs.push(diff);
                    before_iter.next();
                }
                (None, Some(&a_path)) => {
                    // Remaining items on disk are brand-new untracked additions
                    let full_after_path = workdir.join(a_path);
                    let diff = JsonDiff::new(Path::new(""), &full_after_path, a_path, context)?;
                    patch_set.diffs.push(diff);
                    after_iter.next();
                }
                (None, None) => break,
            }
        }

        Ok(patch_set)
    }
}

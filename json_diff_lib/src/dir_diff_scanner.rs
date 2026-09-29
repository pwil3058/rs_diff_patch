// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use crate::{Diff, PatchSet};
use path_utilities::UsefulPathMethods;

pub struct DirDiffScanner;

impl DirDiffScanner {
    /// Recursively scans and collects all file paths within a target root directory,
    /// returning them as sorted relative paths.
    pub fn collect_relative_files(root: &Path) -> io::Result<BTreeSet<PathBuf>> {
        let mut file_set = BTreeSet::new();
        let mut dirs_to_visit = vec![root.to_path_buf()];

        while let Some(current_dir) = dirs_to_visit.pop() {
            // Your usable_dir_entries utility completely cleans up this loop by hiding
            // race conditions and permission logs under the hood!
            for entry in current_dir.usable_dir_entries()? {
                if entry.is_dir() {
                    dirs_to_visit.push(entry.path());
                } else if entry.is_file() {
                    // Normalize the path so it is strictly relative to the workspace root
                    if let Ok(relative_path) = entry.path().strip_prefix(root) {
                        file_set.insert(relative_path.to_path_buf());
                    }
                }
            }
        }

        Ok(file_set)
    }

    /// Compares two directories and generates a unified `PatchSet`.
    pub fn compare_directories(
        before_dir: &Path,
        after_dir: &Path,
        context: u8,
    ) -> io::Result<PatchSet> {
        let mut patch_set = PatchSet::default();

        // 1. Gather files from both states. BTreeSet guarantees they are perfectly sorted lexicographically.
        let before_files = Self::collect_relative_files(before_dir)?;
        let after_files = Self::collect_relative_files(after_dir)?;

        let mut before_iter = before_files.iter().peekable();
        let mut after_iter = after_files.iter().peekable();

        // 2. High-speed two-pointer comparison pass
        loop {
            match (before_iter.peek(), after_iter.peek()) {
                (Some(&b_path), Some(&a_path)) => {
                    if b_path == a_path {
                        // The file exists in both states. Check if its contents changed.
                        let full_before = before_dir.join(b_path);
                        let full_after = after_dir.join(a_path);

                        let diff = Diff::new(&full_before, &full_after, context)?;

                        // We only care if there is an actual structural change inside the file
                        if match_contains_changes(&diff) {
                            patch_set.diffs.push(diff);
                        }

                        before_iter.next();
                        after_iter.next();
                    } else if b_path < a_path {
                        // The 'before' file is missing in the 'after' set -> It was deleted
                        let full_before = before_dir.join(b_path);
                        let diff = Diff::new(&full_before, Path::new(""), context)?;
                        patch_set.diffs.push(diff);
                        before_iter.next();
                    } else {
                        // The 'after' file is brand new -> It was added
                        let full_after = after_dir.join(a_path);
                        let diff = Diff::new(Path::new(""), &full_after, context)?;
                        patch_set.diffs.push(diff);
                        after_iter.next();
                    }
                }
                (Some(&b_path), None) => {
                    // Remaining files in 'before' were all deleted
                    let full_before = before_dir.join(b_path);
                    let diff = Diff::new(&full_before, Path::new(""), context)?;
                    patch_set.diffs.push(diff);
                    before_iter.next();
                }
                (None, Some(&a_path)) => {
                    // Remaining files in 'after' are all brand new additions
                    let full_after = after_dir.join(a_path);
                    let diff = Diff::new(Path::new(""), &full_after, context)?;
                    patch_set.diffs.push(diff);
                    after_iter.next();
                }
                (None, None) => break, // All files successfully evaluated!
            }
        }

        Ok(patch_set)
    }
}

/// Helper to ensure we don't pollute the patch file with empty file changes
fn match_contains_changes(diff: &Diff) -> bool {
    match diff {
        Diff::TextChange(tc) => !tc.is_empty(),
        Diff::ByteChange(bc) => !bc.is_empty(),
        _ => true, // Additions and removals are always valid modifications
    }
}

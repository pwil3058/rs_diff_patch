// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use crate::{Diff, PatchSet};
use path_utilities::UsefulPathMethods;

pub struct DirDiffScanner;

impl DirDiffScanner {
    /// Recursively gathers all paths within a target workspace directory,
    /// sorting and isolating them as strictly relative paths.
    pub fn collect_relative_files(root: &Path) -> io::Result<BTreeSet<PathBuf>> {
        let mut file_set = BTreeSet::new();
        let mut dirs_to_visit = vec![root.to_path_buf()];

        while let Some(current_dir) = dirs_to_visit.pop() {
            for entry in current_dir.usable_dir_entries()? {
                if entry.is_dir() {
                    dirs_to_visit.push(entry.path());
                } else if entry.is_file() {
                    if let Ok(relative_path) = entry.path().strip_prefix(root) {
                        file_set.insert(relative_path.to_path_buf());
                    }
                }
            }
        }

        Ok(file_set)
    }

    /// Evaluates two directory branches side-by-side using an O(N) linear sweep.
    pub fn compare_directories(
        before_dir: &Path,
        after_dir: &Path,
        context: u8,
    ) -> io::Result<PatchSet> {
        let mut patch_set = PatchSet::default();

        let before_files = Self::collect_relative_files(before_dir)?;
        let after_files = Self::collect_relative_files(after_dir)?;

        let mut before_iter = before_files.iter().peekable();
        let mut after_iter = after_files.iter().peekable();

        loop {
            match (before_iter.peek(), after_iter.peek()) {
                (Some(&b_path), Some(&a_path)) => {
                    if b_path == a_path {
                        let diff = Diff::new(before_dir, after_dir, b_path, context)?;
                        if !diff.is_empty() {
                            patch_set.diffs.push(diff);
                        }
                        before_iter.next();
                        after_iter.next();
                    } else if b_path < a_path {
                        let diff = Diff::new(before_dir, after_dir, b_path, context)?;
                        patch_set.diffs.push(diff);
                        before_iter.next();
                    } else {
                        let diff = Diff::new(before_dir, after_dir, a_path, context)?;
                        patch_set.diffs.push(diff);
                        after_iter.next();
                    }
                }
                (Some(&b_path), None) => {
                    let diff = Diff::new(before_dir, after_dir, b_path, context)?;
                    patch_set.diffs.push(diff);
                    before_iter.next();
                }
                (None, Some(&a_path)) => {
                    let diff = Diff::new(before_dir, after_dir, a_path, context)?;
                    patch_set.diffs.push(diff);
                    after_iter.next();
                }
                (None, None) => break,
            }
        }

        Ok(patch_set)
    }
}

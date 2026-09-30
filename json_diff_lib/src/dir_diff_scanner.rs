// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::io;
use std::path::Path;

use crate::{JsonDiff, PatchSet};
use path_utilities::UsefulPathMethods;

pub struct DirDiffScanner;

impl DirDiffScanner {
    /// Evaluates two directory branches side-by-side using an O(N) linear sweep.
    pub fn compare_directories(
        before_dir: impl AsRef<Path>,
        after_dir: impl AsRef<Path>,
        context: u8,
    ) -> io::Result<PatchSet> {
        let before_dir = before_dir.as_ref();
        let after_dir = after_dir.as_ref();
        let mut patch_set = PatchSet::default();

        let before_files = before_dir.collect_relative_files()?;
        let after_files = after_dir.collect_relative_files()?;

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
}

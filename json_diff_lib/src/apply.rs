// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::fs::{self, File};
use std::io::{self, BufWriter};
use std::path::Path;

use crate::JsonDiff;
use crate::PatchSet;
use longest_common_subsequence::sequence::Seq;
use longest_common_subsequence::sequence::SequenceIO;
use pw_diff_lib::apply_bytes::ApplyClumpsClean;
use pw_diff_lib::apply_text::ApplyClumpsFuzzy; // Your strict clean binary patch trait // Your strict clean binary patch trait

/// An atomic transactional patch applicator driver that safely executes patch steps onto a disk destination.
pub struct DirPatchApplier;

impl DirPatchApplier {
    /// Ingests a portable `PatchSet` and safely maps its change layout onto a destination root.
    /// Supports a `reverse` mode flag to perform transactional rolback operations.
    pub fn apply_patch_set(
        target_dir: &Path,
        patch_set: &PatchSet,
        reverse: bool,
    ) -> io::Result<()> {
        for diff in &patch_set.diffs {
            match diff {
                // =========================================================================
                // 1. Text Additions & Removals (Symmetric Layout)
                // =========================================================================
                JsonDiff::TextAdd(path_and_lines) => {
                    let target_path = target_dir.join(path_and_lines.path());
                    if !reverse {
                        // Forward: Create the file
                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let mut file = BufWriter::new(File::create(target_path)?);
                        path_and_lines.write_into(&mut file)?;
                    } else {
                        // Rollback: Remove the added file
                        if target_path.exists() {
                            fs::remove_file(target_path)?;
                        }
                    }
                }
                JsonDiff::TextRemove(path_and_lines) => {
                    let target_path = target_dir.join(path_and_lines.path());
                    if !reverse {
                        // Forward: Delete the file
                        if target_path.exists() {
                            fs::remove_file(target_path)?;
                        }
                    } else {
                        // Rollback: Re-create the deleted file using its stored contents!
                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let mut file = BufWriter::new(File::create(target_path)?);
                        path_and_lines.write_into(&mut file)?;
                    }
                }

                // =========================================================================
                // 2. Binary Additions & Removals (Symmetric Layout)
                // =========================================================================
                JsonDiff::ByteAdd(path_and_bytes) => {
                    let target_path = target_dir.join(path_and_bytes.path());
                    if !reverse {
                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let mut file = BufWriter::new(File::create(target_path)?);
                        path_and_bytes.write_into(&mut file)?;
                    } else {
                        if target_path.exists() {
                            fs::remove_file(target_path)?;
                        }
                    }
                }
                JsonDiff::ByteRemove(path_and_bytes) => {
                    let target_path = target_dir.join(path_and_bytes.path());
                    if !reverse {
                        if target_path.exists() {
                            fs::remove_file(target_path)?;
                        }
                    } else {
                        if let Some(parent) = target_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        let mut file = BufWriter::new(File::create(target_path)?);
                        path_and_bytes.write_into(&mut file)?;
                    }
                }

                // =========================================================================
                // 3. In-Place Modifications (Remain Unchanged)
                // =========================================================================
                JsonDiff::TextChange(text_change_diff) => {
                    let relative_path = diff.path(reverse);
                    let target_path = target_dir.join(relative_path);

                    let current_lines = Seq::<String>::read_from(File::open(&target_path)?)?;
                    let tmp_path = target_path.with_extension("tmp_patch");
                    {
                        let mut output_file = BufWriter::new(File::create(&tmp_path)?);
                        text_change_diff.apply_into(&current_lines, &mut output_file, reverse)?;
                    }
                    fs::rename(tmp_path, target_path)?;
                }
                JsonDiff::ByteChange(byte_change_diff) => {
                    let relative_path = diff.path(reverse);
                    let target_path = target_dir.join(relative_path);

                    let current_bytes = Seq::<u8>::read_from(File::open(&target_path)?)?;
                    let tmp_path = target_path.with_extension("tmp_patch");
                    {
                        let mut output_file = BufWriter::new(File::create(&tmp_path)?);
                        byte_change_diff.apply_into(&current_bytes, &mut output_file, reverse)?;
                    }
                    fs::rename(tmp_path, target_path)?;
                }
            }
        }
        Ok(())
    }
}

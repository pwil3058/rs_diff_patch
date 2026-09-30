// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

pub mod apply;
pub mod binary_diff;
pub mod dir_diff_scanner;
pub mod git_helper;
pub mod text_diff;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, ErrorKind, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::git_helper::{GitComparePair, extract_git_compare_sequences};
use binary_diff::{BinaryChangeDiff, PathAndBytes};
use text_diff::{PathAndLines, TextChangeDiff};

#[derive(Debug, Serialize, Deserialize)]
pub enum JsonDiff {
    TextChange(TextChangeDiff),
    TextAdd(PathAndLines),
    TextRemove(PathAndLines),
    ByteChange(BinaryChangeDiff),
    ByteAdd(PathAndBytes),
    ByteRemove(PathAndBytes),
}

impl JsonDiff {
    pub fn new(
        before_root: &Path,
        after_root: &Path,
        file_path: &Path,
        context: u8,
    ) -> io::Result<Self> {
        let full_before = before_root.join(file_path);
        let full_after = after_root.join(file_path);

        let before_exists = full_before.exists();
        let after_exists = full_after.exists();

        match (before_exists, after_exists) {
            (true, true) => {
                if is_text_file(&full_before)? && is_text_file(&full_after)? {
                    let mut tc = TextChangeDiff::new(&full_before, &full_after, context)?;
                    // Enforce that the stored metadata paths use the portable relative format
                    tc.before_path = file_path.to_path_buf();
                    tc.after_path = file_path.to_path_buf();
                    Ok(Self::TextChange(tc))
                } else {
                    let mut bc = BinaryChangeDiff::new(&full_before, &full_after, context)?;
                    bc.before_path = file_path.to_path_buf();
                    bc.after_path = file_path.to_path_buf();
                    Ok(Self::ByteChange(bc))
                }
            }
            (true, false) => {
                if is_text_file(&full_before)? {
                    let mut pal = PathAndLines::new(&full_before)?;
                    pal.change_path(file_path);
                    Ok(Self::TextRemove(pal))
                } else {
                    let mut pab = PathAndBytes::new(&full_before)?;
                    pab.change_path(file_path);
                    Ok(Self::ByteRemove(pab))
                }
            }
            (false, true) => {
                if is_text_file(&full_after)? {
                    let mut pal = PathAndLines::new(&full_after)?;
                    pal.change_path(file_path);
                    Ok(Self::TextAdd(pal))
                } else {
                    let mut pab = PathAndBytes::new(&full_after)?;
                    pab.change_path(file_path);
                    Ok(Self::ByteAdd(pab))
                }
            }
            (false, false) => Err(io::Error::new(
                ErrorKind::NotFound,
                format!("File path not found in either root: {:?}", file_path),
            )),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            JsonDiff::TextChange(tc) => tc.is_empty(),
            JsonDiff::ByteChange(bc) => bc.is_empty(),
            _ => false, // Additions and removals are always non-empty changes
        }
    }

    pub fn path(&self, reverse: bool) -> &Path {
        match self {
            JsonDiff::TextChange(tc) => {
                if reverse {
                    tc.before_path()
                } else {
                    tc.after_path()
                }
            }
            JsonDiff::ByteChange(bc) => {
                if reverse {
                    &bc.before_path
                } else {
                    &bc.after_path
                }
            }
            JsonDiff::TextAdd(pal) | JsonDiff::TextRemove(pal) => pal.path(),
            JsonDiff::ByteAdd(pab) | JsonDiff::ByteRemove(pab) => pab.path(),
        }
    }
    pub fn generate_from_git<P: AsRef<Path>>(file_path: P, context: u8) -> io::Result<JsonDiff> {
        let path_ref = file_path.as_ref();

        match extract_git_compare_sequences(path_ref)? {
            GitComparePair::Text {
                before,
                before_marker,
                after,
                after_marker,
            } => {
                let changes =
                    longest_common_subsequence::changes::Changes::<String>::new(&before, &after);
                let mut tc = TextChangeDiff::from_changes(path_ref, path_ref, changes, context)?;

                tc.before_marker = Some(before_marker);
                tc.after_marker = Some(after_marker);

                Ok(JsonDiff::TextChange(tc))
            }
            GitComparePair::Binary {
                before,
                before_marker,
                after,
                after_marker,
            } => {
                let changes =
                    longest_common_subsequence::changes::Changes::<u8>::new(&before, &after);
                let mut bc = BinaryChangeDiff::from_changes(path_ref, path_ref, changes, context)?;

                bc.before_marker = Some(before_marker);
                bc.after_marker = Some(after_marker);

                Ok(JsonDiff::ByteChange(bc))
            }
        }
    }
    // pub fn generate_from_git<P: AsRef<Path>>(file_path: P, context: u8) -> io::Result<JsonDiff> {
    //     let path_ref = file_path.as_ref();
    //
    //     match extract_git_compare_sequences(path_ref)? {
    //         GitComparePair::Text { before, after } => {
    //             let changes =
    //                 longest_common_subsequence::changes::Changes::<String>::new(&before, &after);
    //             let tc = TextChangeDiff::from_changes(path_ref, path_ref, changes, context)?;
    //             Ok(JsonDiff::TextChange(tc))
    //         }
    //         GitComparePair::Binary { before, after } => {
    //             let changes =
    //                 longest_common_subsequence::changes::Changes::<u8>::new(&before, &after);
    //             let bc = BinaryChangeDiff::from_changes(path_ref, path_ref, changes, context)?;
    //             Ok(JsonDiff::ByteChange(bc))
    //         }
    //     }
    // }
}

/// Helper function that safely sniffs the first 1024 bytes of a file to classify it.
fn is_text_file(path: &Path) -> io::Result<bool> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 1024];
    let bytes_read = file.read(&mut buffer)?;

    if bytes_read == 0 {
        return Ok(true);
    }

    let sample = &buffer[..bytes_read];
    if sample.contains(&0) {
        return Ok(false);
    }

    let invalid_chars = sample
        .iter()
        .filter(|&&b| b < 7 || (b > 13 && b < 32 && b != 27))
        .count();

    Ok((invalid_chars * 100) / bytes_read < 1)
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PatchSet {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten)]
    pub metadata: HashMap<String, String>,
    pub diffs: Vec<JsonDiff>,
}

impl PatchSet {
    pub fn from_reader<R: io::Read>(reader: R) -> Result<Self, serde_json::Error> {
        let buffered = BufReader::new(reader);
        serde_json::from_reader(buffered)
    }

    pub fn to_writer<W: io::Write>(
        &self,
        writer: W,
        pretty: bool,
    ) -> Result<(), serde_json::Error> {
        let buffered = BufWriter::new(writer);
        if pretty {
            serde_json::to_writer_pretty(buffered, self)
        } else {
            serde_json::to_writer(buffered, self)
        }
    }
}

#[cfg(test)]
mod tests;

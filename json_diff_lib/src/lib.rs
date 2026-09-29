// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

pub mod apply;
pub mod dir_diff_scanner;

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, ErrorKind, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};

use json_binary_diff_lib::{BinaryChangeDiff, PathAndBytes};
use json_text_diff_lib::{PathAndLines, TextChangeDiff};

#[derive(Debug, Serialize, Deserialize)]
pub enum Diff {
    TextChange(TextChangeDiff),
    TextAdd(PathAndLines),
    TextRemove(PathAndLines),
    ByteChange(BinaryChangeDiff),
    ByteAdd(PathAndBytes),
    ByteRemove(PathAndBytes),
}

impl Diff {
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
            Diff::TextChange(tc) => tc.is_empty(),
            Diff::ByteChange(bc) => bc.is_empty(),
            _ => false, // Additions and removals are always non-empty changes
        }
    }

    pub fn path(&self, reverse: bool) -> &Path {
        match self {
            Diff::TextChange(tc) => {
                if reverse {
                    tc.before_path()
                } else {
                    tc.after_path()
                }
            }
            Diff::ByteChange(bc) => {
                if reverse {
                    &bc.before_path
                } else {
                    &bc.after_path
                }
            }
            Diff::TextAdd(pal) | Diff::TextRemove(pal) => pal.path(),
            Diff::ByteAdd(pab) | Diff::ByteRemove(pab) => pab.path(),
        }
    }
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
    pub diffs: Vec<Diff>,
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

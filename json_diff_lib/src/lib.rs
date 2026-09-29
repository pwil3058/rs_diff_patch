// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

pub mod dir_diff_scanner;

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufReader, BufWriter, ErrorKind, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};

use json_binary_diff_lib::{BinaryChangeDiff as ByteChangeDiff, PathAndBytes};
use json_text_diff_lib::{PathAndLines, TextChangeDiff}; // Named to match your binary module

#[derive(Debug, Serialize, Deserialize)]
pub enum Diff {
    TextChange(TextChangeDiff),
    TextAdd(PathAndLines),
    TextRemove(PathAndLines),
    ByteChange(ByteChangeDiff),
    ByteAdd(PathAndBytes),
    ByteRemove(PathAndBytes),
}

impl Diff {
    pub fn new(before_file_path: &Path, after_file_path: &Path, context: u8) -> io::Result<Self> {
        let before_exists = before_file_path.exists();
        let after_exists = after_file_path.exists();

        match (before_exists, after_exists) {
            (true, true) => {
                // Pre-flight check: Determine type dynamically without speculative allocations
                if is_text_file(before_file_path)? && is_text_file(after_file_path)? {
                    match TextChangeDiff::new(before_file_path, after_file_path, context) {
                        Ok(text_change_diff) => Ok(Self::TextChange(text_change_diff)),
                        Err(_) => Ok(Self::ByteChange(ByteChangeDiff::new(
                            before_file_path,
                            after_file_path,
                            context,
                        )?)),
                    }
                } else {
                    Ok(Self::ByteChange(ByteChangeDiff::new(
                        before_file_path,
                        after_file_path,
                        context,
                    )?))
                }
            }
            (true, false) => {
                if is_text_file(before_file_path)? {
                    match PathAndLines::new(before_file_path) {
                        Ok(path_and_lines) => Ok(Self::TextRemove(path_and_lines)),
                        Err(_) => Ok(Self::ByteRemove(PathAndBytes::new(before_file_path)?)),
                    }
                } else {
                    Ok(Self::ByteRemove(PathAndBytes::new(before_file_path)?))
                }
            }
            (false, true) => {
                // Preserve after_file_path so the patching framework knows where to create the new asset
                if is_text_file(after_file_path)? {
                    match PathAndLines::new(after_file_path) {
                        Ok(path_and_lines) => Ok(Self::TextAdd(path_and_lines)),
                        Err(_) => Ok(Self::ByteAdd(PathAndBytes::new(after_file_path)?)),
                    }
                } else {
                    Ok(Self::ByteAdd(PathAndBytes::new(after_file_path)?))
                }
            }
            (false, false) => Err(io::Error::new(
                ErrorKind::NotFound,
                format!(
                    "Neither path exists: {:?} or {:?}",
                    before_file_path, after_file_path
                ),
            )),
        }
    }

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

/// Helper function that safely sniffs the first 1024 bytes of a file to classify it.
/// Returns true if text-like, or false if binary control structures dominate.
fn is_text_file(path: &Path) -> io::Result<bool> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 1024];
    let bytes_read = file.read(&mut buffer)?;

    if bytes_read == 0 {
        return Ok(true); // Treat empty files safely as text lines
    }

    let sample = &buffer[..bytes_read];

    // Check for null characters or unexpected non-text control blocks
    if sample.contains(&0) {
        return Ok(false);
    }

    // Heuristic counter tracking non-printable character layouts
    let invalid_chars = sample
        .iter()
        .filter(|&&b| b < 7 || (b > 13 && b < 32 && b != 27))
        .count();

    // If more than 1% of the characters are non-printable control codes, treat as binary
    Ok((invalid_chars * 100) / bytes_read < 1)
}

// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PatchSet {
    // 1. Explicitly named common fields, completely optional to omit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    // 2. This dynamically absorbs Linux/Git-style variables (author, signed_off, etc.)
    // without forcing you to pre-define them inside a rigid struct.
    #[serde(flatten)]
    pub metadata: HashMap<String, String>,

    // 3. The actual master collection of file changes, additions, and removals
    pub diffs: Vec<Diff>,
}

impl PatchSet {
    /// Loads a PatchSet from an input stream using an optimized buffer reader interface.
    pub fn from_reader<R: io::Read>(reader: R) -> Result<Self, serde_json::Error> {
        let buffered = BufReader::new(reader);
        serde_json::from_reader(buffered)
    }

    /// Serializes the entire patch suite directly onto a disk handle.
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

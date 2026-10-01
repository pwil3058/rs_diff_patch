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

use crate::git_helper::GitComparePair;
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
    ) -> io::Result<Option<Self>> {
        let full_before = before_root.join(file_path);
        let full_after = after_root.join(file_path);

        let before_exists = full_before.exists();
        let after_exists = full_after.exists();

        match (before_exists, after_exists) {
            (true, true) => {
                if is_text_file(&full_before)? && is_text_file(&full_after)? {
                    if let Some(tc) =
                        TextChangeDiff::new(before_root, after_root, file_path, context)?
                    {
                        Ok(Some(Self::TextChange(tc)))
                    } else {
                        Ok(None)
                    }
                } else {
                    if let Some(bc) =
                        BinaryChangeDiff::new(before_root, after_root, file_path, context)?
                    {
                        Ok(Some(Self::ByteChange(bc)))
                    } else {
                        Ok(None)
                    }
                }
            }
            (true, false) => {
                if is_text_file(&full_before)? {
                    let mut pal = PathAndLines::new(&full_before)?;
                    pal.change_path(file_path);
                    Ok(Some(Self::TextRemove(pal)))
                } else {
                    let mut pab = PathAndBytes::new(&full_before)?;
                    pab.change_path(file_path);
                    Ok(Some(Self::ByteRemove(pab)))
                }
            }
            (false, true) => {
                if is_text_file(&full_after)? {
                    let mut pal = PathAndLines::new(&full_after)?;
                    pal.change_path(file_path);
                    Ok(Some(Self::TextAdd(pal)))
                } else {
                    let mut pab = PathAndBytes::new(&full_after)?;
                    pab.change_path(file_path);
                    Ok(Some(Self::ByteAdd(pab)))
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
                    &bc.before.path
                } else {
                    &bc.after.path
                }
            }
            JsonDiff::TextAdd(pal) | JsonDiff::TextRemove(pal) => pal.path(),
            JsonDiff::ByteAdd(pab) | JsonDiff::ByteRemove(pab) => pab.path(),
        }
    }

    pub fn generate_from_git<P: AsRef<Path>>(
        file_path: P,
        context: u8,
    ) -> io::Result<Option<JsonDiff>> {
        let absolute_path = std::fs::canonicalize(file_path.as_ref())?;

        let repo = git2::Repository::discover(&absolute_path)
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
        let workdir = repo.workdir().unwrap();

        let relative_path = absolute_path
            .strip_prefix(workdir)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?
            .to_path_buf();

        match crate::git_helper::extract_git_compare_sequences(&absolute_path)? {
            GitComparePair::Text {
                before,
                before_marker,
                after,
                after_marker,
            } => {
                let changes =
                    longest_common_subsequence::changes::Changes::<String>::new(&before, &after);

                let before_meta = crate::file_meta::FileMeta {
                    path: relative_path.clone(),
                    marker: Some(before_marker),
                };
                let after_meta = crate::file_meta::FileMeta {
                    path: relative_path,
                    marker: Some(after_marker),
                };

                if let Some(tc) =
                    TextChangeDiff::from_changes(before_meta, after_meta, changes, context)
                {
                    Ok(Some(JsonDiff::TextChange(tc)))
                } else {
                    Ok(None)
                }
            }
            GitComparePair::Binary {
                before,
                before_marker,
                after,
                after_marker,
            } => {
                let changes =
                    longest_common_subsequence::changes::Changes::<u8>::new(&before, &after);

                let before_meta = crate::file_meta::FileMeta {
                    path: relative_path.clone(),
                    marker: Some(before_marker),
                };
                let after_meta = crate::file_meta::FileMeta {
                    path: relative_path,
                    marker: Some(after_marker),
                };

                if let Some(bc) =
                    BinaryChangeDiff::from_changes(before_meta, after_meta, changes, context)
                {
                    Ok(Some(JsonDiff::ByteChange(bc)))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Generates a diff between two specific historical commit states for a target file.
    pub fn generate_from_git_commits<P: AsRef<Path>>(
        file_path: P,
        before_commit_id: &str,
        after_commit_id: &str,
        context: u8,
    ) -> io::Result<Option<Self>> {
        let path_ref = file_path.as_ref();

        // Query the historical blobs from the repository context
        match crate::git_helper::extract_git_commit_compare_sequences(
            path_ref,
            before_commit_id,
            after_commit_id,
        )? {
            GitComparePair::Text {
                before,
                before_marker,
                after,
                after_marker,
            } => {
                let changes =
                    longest_common_subsequence::changes::Changes::<String>::new(&before, &after);

                // Isolate the clean repository-relative path key format
                let relative_path = if path_ref.is_absolute() {
                    let repo = git2::Repository::discover(path_ref)
                        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
                    let workdir = repo.workdir().unwrap();
                    path_ref
                        .strip_prefix(workdir)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?
                        .to_path_buf()
                } else {
                    path_ref.to_path_buf()
                };

                // Package pre-packaged commit markers into proper FileMeta entries!
                let before_meta = crate::file_meta::FileMeta {
                    path: relative_path.clone(),
                    marker: Some(before_marker),
                };
                let after_meta = crate::file_meta::FileMeta {
                    path: relative_path,
                    marker: Some(after_marker),
                };

                if let Some(tc) =
                    TextChangeDiff::from_changes(before_meta, after_meta, changes, context)
                {
                    Ok(Some(JsonDiff::TextChange(tc)))
                } else {
                    Ok(None)
                }
            }
            GitComparePair::Binary {
                before,
                before_marker,
                after,
                after_marker,
            } => {
                let changes =
                    longest_common_subsequence::changes::Changes::<u8>::new(&before, &after);

                let relative_path = if path_ref.is_absolute() {
                    let repo = git2::Repository::discover(path_ref)
                        .map_err(|e| io::Error::new(io::ErrorKind::NotFound, e.to_string()))?;
                    let workdir = repo.workdir().unwrap();
                    path_ref.strip_prefix(workdir).unwrap().to_path_buf()
                } else {
                    path_ref.to_path_buf()
                };

                let before_meta = crate::file_meta::FileMeta {
                    path: relative_path.clone(),
                    marker: Some(before_marker),
                };
                let after_meta = crate::file_meta::FileMeta {
                    path: relative_path,
                    marker: Some(after_marker),
                };

                if let Some(bc) =
                    BinaryChangeDiff::from_changes(before_meta, after_meta, changes, context)
                {
                    Ok(Some(JsonDiff::ByteChange(bc)))
                } else {
                    Ok(None)
                }
            }
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

mod file_meta;
#[cfg(test)]
mod tests;

// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents type-safe file tracking metadata variants that serialize cleanly into JSON format.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum FileMarker {
    /// A historical Git commit baseline identifier, holding a shortened 7-character OID hash.
    /// Rendered as: "Commit": "a1c3e5f"
    Commit(String),
    /// A standard filesystem modification timestamp formatted as an explicit local date-time string.
    /// Rendered as: "Modified": "2026-09-30 13:02:15"
    Modified(String),
    /// Indicates a file is a brand-new asset present on disk but untracked by the repository index.
    /// Rendered as: "Untracked": true
    Untracked,
}

/// A unified metadata tracking container linking a file's repository-relative path
/// with its historical or physical state descriptor marker.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct FileMeta {
    /// The portable, repository-relative destination path of the target file.
    pub path: PathBuf,
    /// An optional structural status indicator (Commit hash, Timestamp, or Untracked status).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<FileMarker>,
}

impl FileMeta {
    /// Scenario A: Standard Plain Filesystem File. Extracts local file modification time.
    pub fn from_plain_file(root: &Path, file_path: &Path) -> Self {
        let full_path = root.join(file_path);
        let marker = if full_path.exists() {
            if let Ok(metadata) = fs::metadata(&full_path) {
                if let Ok(modified_time) = metadata.modified() {
                    let datetime: chrono::DateTime<chrono::Local> = modified_time.into();
                    Some(FileMarker::Modified(
                        datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
                    ))
                } else {
                    Some(FileMarker::Modified(String::from(
                        "Unknown Modification Time",
                    )))
                }
            } else {
                Some(FileMarker::Untracked)
            }
        } else {
            Some(FileMarker::Untracked)
        };

        Self {
            path: file_path.to_path_buf(),
            marker,
        }
    }

    /// Scenario B: Historical Git Commit Blob. Captures a shortened 7-character commit OID hash.
    pub fn from_git_commit(file_path: &Path, commit_oid: git2::Oid) -> Self {
        let full_id = commit_oid.to_string();
        let short_id = if full_id.len() >= 7 {
            &full_id[..7]
        } else {
            &full_id
        };

        Self {
            path: file_path.to_path_buf(),
            marker: Some(FileMarker::Commit(short_id.to_string())),
        }
    }

    /// Scenario C: Active Checked-out Git Workspace File on Disk. Extracts current modification time.
    pub fn from_git_workspace_file(file_path: &Path, full_absolute_disk_path: &Path) -> Self {
        let marker = if full_absolute_disk_path.exists() {
            if let Ok(metadata) = fs::metadata(full_absolute_disk_path) {
                if let Ok(modified_time) = metadata.modified() {
                    let datetime: chrono::DateTime<chrono::Local> = modified_time.into();
                    Some(FileMarker::Modified(
                        datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
                    ))
                } else {
                    Some(FileMarker::Modified(String::from(
                        "Unknown Modification Time",
                    )))
                }
            } else {
                Some(FileMarker::Modified(String::from(
                    "Unknown Modification Time",
                )))
            }
        } else {
            Some(FileMarker::Untracked)
        };

        Self {
            path: file_path.to_path_buf(),
            marker,
        }
    }

    /// Scenario D: Non-existent, missing, or purely virtual asset path context stub.
    pub fn empty() -> Self {
        Self {
            path: PathBuf::new(),
            marker: Some(FileMarker::Untracked),
        }
    }
}

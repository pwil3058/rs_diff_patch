// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use git2::Repository;

use longest_common_subsequence::sequence::{Seq, SequenceIO};

/// Beautiful, type-safe file tracking indicators that render cleanly in JSON.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum FileMarker {
    /// Rendered as: "Commit": "a1c3e5f"
    Commit(String),
    /// Rendered as: "Modified": "2026-09-30 13:02:15"
    Modified(String),
    /// Rendered as: "Untracked": true
    Untracked,
}

/// Dynamic container holding extracted sequence blocks alongside their clear type-safe markers
pub enum GitComparePair {
    Text {
        before: Seq<String>,
        before_marker: FileMarker,
        after: Seq<String>,
        after_marker: FileMarker,
    },
    Binary {
        before: Seq<u8>,
        before_marker: FileMarker,
        after: Seq<u8>,
        after_marker: FileMarker,
    },
}

pub fn extract_git_compare_sequences<P: AsRef<Path>>(file_path: P) -> io::Result<GitComparePair> {
    let absolute_path = std::fs::canonicalize(file_path.as_ref())?;

    let repo = Repository::discover(&absolute_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Git repository not found: {}", e),
        )
    })?;

    let workdir = repo.workdir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Bare repositories do not have checked-out work paths",
        )
    })?;
    let relative_git_path = absolute_path.strip_prefix(workdir).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Path sits outside work tree: {}", e),
        )
    })?;

    let index = repo.index().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to read git index: {}", e),
        )
    })?;

    // 1. Resolve Before Marker: Staged index blob maps to a clean short commit OID slice
    let mut before_marker = FileMarker::Untracked;
    let before_bytes = match index.get_path(relative_git_path, 0) {
        Some(entry) => {
            let blob = repo.find_blob(entry.id).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Git blob corruption: {}", e))
            })?;
            let full_id = blob.id().to_string();
            // Isolate short 7-character commit-oid string slice format safely
            let short_id = if full_id.len() >= 7 {
                &full_id[..7]
            } else {
                &full_id
            };
            before_marker = FileMarker::Commit(short_id.to_string());
            blob.content().to_vec()
        }
        None => vec![],
    };

    // 2. Resolve After Marker: Local checked-out file converts to a human-readable local date string
    let mut after_marker = FileMarker::Untracked;
    let mut after_bytes = Vec::new();
    if absolute_path.exists() {
        let metadata = std::fs::metadata(&absolute_path)?;
        if let Ok(modified_time) = metadata.modified() {
            // Translate file system time into a clear readable calendar string representation
            // e.g., using standard datetime library formatting or fallback string translations
            let datetime: chrono::DateTime<chrono::Local> = modified_time.into();
            after_marker = FileMarker::Modified(datetime.format("%Y-%m-%d %H:%M:%S").to_string());
        } else {
            after_marker = FileMarker::Modified(String::from("Unknown Modification Time"));
        }

        let mut file = BufReader::new(File::open(&absolute_path)?);
        file.read_to_end(&mut after_bytes)?;
    }

    if is_text_buffer(&before_bytes) && is_text_buffer(&after_bytes) {
        let before_seq = Seq::<String>::read_from(&before_bytes[..])?;
        let after_seq = Seq::<String>::read_from(&after_bytes[..])?;
        Ok(GitComparePair::Text {
            before: before_seq,
            before_marker,
            after: after_seq,
            after_marker,
        })
    } else {
        Ok(GitComparePair::Binary {
            before: Seq::from(before_bytes),
            before_marker,
            after: Seq::from(after_bytes),
            after_marker,
        })
    }
}

/// Extracts a file's contents at two specific historical points in time directly from Git's object database.
pub fn extract_git_commit_compare_sequences<P: AsRef<Path>>(
    file_path: P,
    before_commit_id: &str,
    after_commit_id: &str,
) -> io::Result<GitComparePair> {
    let absolute_path = std::fs::canonicalize(file_path.as_ref())?;

    // 1. Discover the enclosing Git repository from the target file location
    let repo = Repository::discover(&absolute_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Git repository not found: {}", e),
        )
    })?;

    // 2. Resolve the relative repository path required by Git tree queries
    let workdir = repo.workdir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Bare repositories do not have checked-out work paths",
        )
    })?;
    let relative_git_path = absolute_path.strip_prefix(workdir).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Path sits outside work tree: {}", e),
        )
    })?;

    // 3. Helper to look up file contents (blob) at a specific commit string handle (hash/branch/tag)
    let fetch_blob_bytes = |commit_str: &str| -> io::Result<(Vec<u8>, String)> {
        // Resolve the freeform string (like "HEAD" or a short hash) into a concrete Git object reference
        let object = repo.revparse_single(commit_str).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Could not resolve Git reference '{}': {}", commit_str, e),
            )
        })?;

        let commit = object.as_commit().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Reference '{}' does not resolve to a valid commit",
                    commit_str
                ),
            )
        })?;

        // Navigate down into the commit's root directory tree lookup layout
        let tree = commit.tree().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to read commit tree: {}", e),
            )
        })?;

        let entry = tree.get_path(relative_git_path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "Path '{:?}' not found in commit '{}': {}",
                    relative_git_path, commit_str, e
                ),
            )
        })?;

        let blob = repo.find_blob(entry.id()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Git blob lookup failure: {}", e),
            )
        })?;

        // Format a human-friendly short 7-character commit ID hash string
        let full_id = commit.id().to_string();
        let short_id = if full_id.len() >= 7 {
            &full_id[..7]
        } else {
            &full_id
        };

        Ok((blob.content().to_vec(), short_id.to_string()))
    };

    // 4. Ingest both snapshots directly out of memory
    let (before_bytes, before_short_id) = fetch_blob_bytes(before_commit_id)?;
    let (after_bytes, after_short_id) = fetch_blob_bytes(after_commit_id)?;

    let before_marker = FileMarker::Commit(before_short_id);
    let after_marker = FileMarker::Commit(after_short_id);

    // 5. Package into the correct type-safe container variants
    if crate::git_helper::is_text_buffer(&before_bytes)
        && crate::git_helper::is_text_buffer(&after_bytes)
    {
        let before_seq = Seq::<String>::read_from(&before_bytes[..])?;
        let after_seq = Seq::<String>::read_from(&after_bytes[..])?;
        Ok(GitComparePair::Text {
            before: before_seq,
            before_marker,
            after: after_seq,
            after_marker,
        })
    } else {
        Ok(GitComparePair::Binary {
            before: Seq::from(before_bytes),
            before_marker,
            after: Seq::from(after_bytes),
            after_marker,
        })
    }
}

/// Traverses a Git commit tree recursively and collects all file paths as a sorted BTreeSet.
pub fn collect_files_from_commit(
    repo: &Repository,
    commit_str: &str,
) -> io::Result<BTreeSet<PathBuf>> {
    let object = repo.revparse_single(commit_str).map_err(|e| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Could not resolve Git reference '{}': {}", commit_str, e),
        )
    })?;

    let commit = object.as_commit().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Reference '{}' is not a commit", commit_str),
        )
    })?;

    let tree = commit.tree().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to read commit tree: {}", e),
        )
    })?;

    let mut file_set = BTreeSet::new();

    // Use git2's built-in tree walker to handle the recursion with zero allocation overhead
    tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if let Ok(name) = entry.name() {
            if entry.kind() == Some(git2::ObjectType::Blob) {
                let mut path = PathBuf::from(root);
                path.push(name);
                file_set.insert(path);
            }
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Tree walk failure: {}", e)))?;

    Ok(file_set)
}

/// Lightweight, allocations-free helper checking buffer content signatures for control codes.
fn is_text_buffer(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return true;
    }
    if bytes.contains(&0) {
        return false;
    } // Null terminator denotes true binary data
    let invalid_count = bytes
        .iter()
        .filter(|&&b| b < 7 || (b > 13 && b < 32 && b != 27))
        .count();
    (invalid_count * 100) / bytes.len() < 1
}

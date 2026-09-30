// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::fs::File;
use std::io;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use path_utilities::*;

use crate::git_helper::FileMarker;
use longest_common_subsequence::{
    changes::*,
    range::Range,
    sequence::{Seq, SequenceIO},
};
use pw_diff_lib::apply_text::{ApplyClumpFuzzy, ApplyClumpsFuzzy};
use pw_diff_lib::snippet::SnippetIfce;
use pw_diff_lib::{
    apply_text::TextClumpBasics,
    snippet::{ExtractSnippet, TextSnippet},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TextChangeClump {
    context_lengths: (u8, u8),
    before: TextSnippet,
    after: TextSnippet,
}

impl From<ChangeClump<'_, String>> for TextChangeClump {
    fn from(change_clump: ChangeClump<String>) -> Self {
        let (before_range, after_range) = change_clump.ranges();

        TextChangeClump {
            context_lengths: change_clump.context_lengths(),
            before: change_clump.before.extract_snippet(before_range),
            after: change_clump.after.extract_snippet(after_range),
        }
    }
}

impl ChangeBasics for TextChangeClump {
    #[inline]
    fn before_start(&self, reverse: bool) -> usize {
        if reverse {
            self.after.start
        } else {
            self.before.start
        }
    }

    #[inline]
    fn before_end(&self, reverse: bool) -> usize {
        if reverse {
            self.after.start + self.after.items.len()
        } else {
            self.before.start + self.before.items.len()
        }
    }
}

impl TextClumpBasics for TextChangeClump {
    #[inline]
    fn context_lengths(&self) -> (u8, u8) {
        self.context_lengths
    }

    #[inline]
    fn before_lines(&self, range: Option<Range>, reverse: bool) -> impl Iterator<Item = &String> {
        if reverse {
            self.after.items(range)
        } else {
            self.before.items(range)
        }
    }
}

impl TextChangeClump {
    #[inline]
    pub fn before(&self, reverse: bool) -> &TextSnippet {
        if reverse { &self.after } else { &self.before }
    }

    #[inline]
    pub fn after(&self, reverse: bool) -> &TextSnippet {
        if reverse { &self.before } else { &self.after }
    }
}

impl ApplyClumpFuzzy for TextChangeClump {}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TextChangeDiff {
    pub before_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_marker: Option<FileMarker>, // 👈 Updated property hook

    pub after_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_marker: Option<FileMarker>,

    clumps: Vec<TextChangeClump>,
}

impl TextChangeDiff {
    pub fn new(before_file_path: &Path, after_file_path: &Path, context: u8) -> io::Result<Self> {
        let before_lines = Seq::<String>::read_from(BufReader::new(File::open(before_file_path)?))?;
        let after_lines = Seq::<String>::read_from(BufReader::new(File::open(after_file_path)?))?;
        let changes = Changes::<String>::new(&before_lines, &after_lines);

        let before_marker = if before_file_path.exists() {
            if let Ok(metadata) = std::fs::metadata(before_file_path) {
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

        let after_marker = if after_file_path.exists() {
            if let Ok(metadata) = std::fs::metadata(after_file_path) {
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

        Ok(Self {
            before_path: before_file_path.relative_path_buf().unwrap(),
            before_marker,
            after_path: after_file_path.relative_path_buf().unwrap(),
            after_marker,
            clumps: changes
                .change_clumps(context)
                .map(TextChangeClump::from)
                .collect(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.clumps.is_empty()
    }

    #[inline]
    pub fn from_reader<R: io::Read>(reader: &mut R) -> Result<Self, serde_json::Error> {
        let buffered = BufReader::new(reader);
        serde_json::from_reader(buffered)
    }

    #[inline]
    pub fn before_path(&self) -> &Path {
        &self.before_path
    }

    #[inline]
    pub fn after_path(&self) -> &Path {
        &self.after_path
    }

    pub fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }

    pub fn from_changes(
        before_path_ref: impl AsRef<Path>,
        after_path_ref: impl AsRef<Path>,
        changes: Changes<String>,
        context: u8,
    ) -> io::Result<Self> {
        let before_path = before_path_ref.as_ref().to_path_buf();
        let after_path = after_path_ref.as_ref().to_path_buf();

        let before_marker = if before_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&before_path) {
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

        let after_marker = if after_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&after_path) {
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

        let clumps = changes
            .change_clumps(context)
            .map(TextChangeClump::from)
            .collect();

        Ok(Self {
            before_path,
            before_marker,
            after_path,
            after_marker,
            clumps,
        })
    }
}

impl ApplyClumpsFuzzy<TextChangeClump> for TextChangeDiff {
    #[inline]
    fn clumps<'s>(&'s self) -> impl Iterator<Item = &'s TextChangeClump>
    where
        TextChangeClump: 's,
    {
        self.clumps.iter()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PathAndLines {
    path: PathBuf,
    lines: Box<[String]>,
}

impl PathAndLines {
    pub fn new(path: &Path) -> io::Result<Self> {
        use std::io::BufRead;
        let mut lines = vec![];
        let mut reader = io::BufReader::new(File::open(path)?);

        let mut line_buf = String::new();
        while reader.read_line(&mut line_buf)? > 0 {
            lines.push(line_buf.clone());
            line_buf.clear();
        }

        Ok(Self {
            path: path.to_path_buf(),
            lines: lines.into_boxed_slice(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn change_path(&mut self, new_path: &Path) {
        self.path = new_path.to_path_buf()
    }

    pub fn write_into<W: io::Write>(&self, into: &mut W) -> io::Result<()> {
        for line in self.lines.iter() {
            into.write_all(line.as_bytes())?;
        }
        Ok(())
    }
}

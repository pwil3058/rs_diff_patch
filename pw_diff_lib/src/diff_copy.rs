// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::byte_diff_copy::{ByteChangeDiff, PathAndBytes};
use crate::text_diff_copy::{PathAndLines, TextChangeDiff};
use serde::{Deserialize, Serialize};
use std::io;
use std::io::ErrorKind;
use std::path::Path;

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
        if before_file_path.exists() {
            if after_file_path.exists() {
                match TextChangeDiff::new(before_file_path, after_file_path, context) {
                    Ok(text_change_diff) => Ok(Self::TextChange(text_change_diff)),
                    Err(_) => Ok(Self::ByteChange(ByteChangeDiff::new(
                        before_file_path,
                        after_file_path,
                        context,
                    )?)),
                }
            } else {
                match PathAndLines::new(before_file_path) {
                    Ok(path_and_lines) => Ok(Self::TextRemove(path_and_lines)),
                    Err(_) => Ok(Self::ByteRemove(PathAndBytes::new(before_file_path)?)),
                }
            }
        } else if after_file_path.exists() {
            match PathAndLines::new(after_file_path) {
                Ok(mut path_and_lines) => {
                    path_and_lines.change_path(&before_file_path);
                    Ok(Self::TextAdd(path_and_lines))
                }
                Err(_) => {
                    let mut path_and_bytes = PathAndBytes::new(after_file_path)?;
                    path_and_bytes.change_path(&before_file_path);
                    Ok(Self::ByteAdd(path_and_bytes))
                }
            }
        } else {
            Err(io::Error::new(
                ErrorKind::NotFound,
                "Neither input file exists!",
            ))
        }
    }

    pub fn from_reader<R: io::Read>(reader: &mut R) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(reader)
    }

    pub fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }
}

#[cfg(test)]
mod diff_tests;

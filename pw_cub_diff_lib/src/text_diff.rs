// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::error::Error;
use std::num::ParseIntError;
use std::slice::Iter;
use std::{fmt, io};

use pw_diff_lib::data::*;
use pw_diff_lib::range::{Len, Range};
use regex::Captures;

use crate::git_binary_diff::git_delta::DeltaError;
use crate::DiffFormat;

#[derive(Debug)]
pub enum DiffParseError {
    MissingAfterFileData(usize),
    ParseNumberError(ParseIntError, usize),
    UnexpectedEndOfInput,
    UnexpectedEndHunk(DiffFormat, usize),
    UnexpectedInput(DiffFormat, String),
    SyntaxError(DiffFormat, usize),
    Base85Error(String),
    ZLibInflateError(String),
    GitDeltaError(DeltaError),
    IOError(io::Error),
}

impl fmt::Display for DiffParseError {
    // TODO: flesh out fmt::Display implementation for DiffParseError
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bug the developer to fix this!")
    }
}

impl Error for DiffParseError {
    // TODO: flesh out error::Error implementation for DiffParseError
    fn description(&self) -> &str {
        "I'm the superhero of diff parsing errors"
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

pub type DiffParseResult<T> = Result<T, DiffParseError>;

pub trait CheckEndOfInput<T> {
    fn check_end_of_input(&self) -> DiffParseResult<&T>;
}

impl<T> CheckEndOfInput<T> for Option<T> {
    fn check_end_of_input(&self) -> DiffParseResult<&T> {
        match self {
            Some(t) => Ok(t),
            None => Err(DiffParseError::UnexpectedEndOfInput),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PathAndTimestamp {
    pub file_path: String,
    pub time_stamp: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct StartAndLength {
    pub start: usize,
    pub length: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub struct StartsAndLengths {
    pub before: StartAndLength,
    pub after: StartAndLength,
}

#[derive(Debug)]
pub struct TextDiffHeader {
    pub lines: Vec<String>,
    pub ante_pat: PathAndTimestamp,
    pub post_pat: PathAndTimestamp,
}

pub trait TextDiffHunk {
    fn len(&self) -> usize;
    fn iter(&self) -> Iter<String>;

    fn ante_lines(&self) -> Vec<String>;
    fn post_lines(&self) -> Vec<String>;

    fn adds_trailing_white_space(&self) -> bool;

    //   fn get_abstract_diff_hunk(&self) -> AbstractHunk;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct TextDiff<H: TextDiffHunk> {
    lines_consumed: usize, // time saver
    header: TextDiffHeader,
    hunks: Vec<H>,
}

pub trait TextDiffParser<H: TextDiffHunk> {
    fn new() -> Self;
    fn ante_file_rec<'t>(&self, line: &'t str) -> Option<Captures<'t>>;
    fn post_file_rec<'t>(&self, line: &'t str) -> Option<Captures<'t>>;
    fn get_hunk_at(&self, lines: &Data<String>, index: usize) -> DiffParseResult<Option<H>>;

    fn _get_file_data_fm_captures(&self, captures: &Captures) -> PathAndTimestamp {
        let file_path = if let Some(path) = captures.get(2) {
            path.as_str()
        } else {
            captures.get(3).unwrap().as_str() // TODO: confirm unwrap is OK here
        };
        let time_stamp = captures.get(4).map(|ts| ts.as_str().to_string());
        PathAndTimestamp {
            file_path: file_path.to_string(),
            time_stamp,
        }
    }

    fn get_text_diff_header_at(
        &self,
        lines: &Data<String>,
        start_index: usize,
    ) -> DiffParseResult<Option<TextDiffHeader>> {
        let mut iter = lines.subsequence_from(start_index);
        let ante_pat = {
            if let Some(line) = iter.next() {
                if let Some(captures) = self.ante_file_rec(line) {
                    self._get_file_data_fm_captures(&captures)
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        };
        let post_pat = {
            if let Some(line) = iter.next() {
                if let Some(captures) = self.ante_file_rec(line) {
                    self._get_file_data_fm_captures(&captures)
                } else {
                    return Err(DiffParseError::MissingAfterFileData(start_index));
                }
            } else {
                return Err(DiffParseError::MissingAfterFileData(start_index));
            }
        };
        let lines = lines
            .subsequence(Range(start_index, start_index + 2))
            .map(|s| s.to_string())
            .collect();
        Ok(Some(TextDiffHeader {
            lines,
            ante_pat,
            post_pat,
        }))
    }

    fn get_diff_at(
        &self,
        lines: &Data<String>,
        start_index: usize,
    ) -> DiffParseResult<Option<TextDiff<H>>> {
        if lines.len() - start_index < 2 {
            return Ok(None);
        }
        let mut index = start_index;
        let header = if let Some(header) = self.get_text_diff_header_at(lines, index)? {
            index += header.lines.len();
            header
        } else {
            return Ok(None);
        };
        let mut hunks: Vec<H> = Vec::new();
        while index < lines.len() {
            if let Some(hunk) = self.get_hunk_at(lines, index)? {
                index += hunk.len();
                hunks.push(hunk);
            } else {
                break;
            }
        }
        let diff = TextDiff::<H> {
            lines_consumed: index - start_index,
            header,
            hunks,
        };
        Ok(Some(diff))
    }
}

pub fn extract_source_lines<F: Fn(&str) -> bool>(
    lines: &[String],
    trim_left_n: usize,
    skip: F,
) -> Vec<String> {
    let mut trimmed_lines: Vec<String> = vec![];
    for (index, line) in lines.iter().enumerate() {
        if skip(line) || line.starts_with('\\') {
            continue;
        }
        if (index + 1) == lines.len() || !lines[index + 1].starts_with('\\') {
            trimmed_lines.push(line[trim_left_n..].to_string());
        } else {
            trimmed_lines.push(line[trim_left_n..].trim_end_matches('\n').to_string());
        }
    }
    trimmed_lines
}

// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::fmt;
use std::io;
use std::slice::Iter;
use std::str::FromStr;

use inflate;
use regex::Regex;

pub mod git_base85;
pub mod git_delta;

use crate::DiffFormat;
use crate::lines::{Line, Lines};
use crate::text_diff::{DiffParseError, DiffParseResult};

#[derive(Debug)]
pub enum GitBinaryDiffMethod {
    Delta,
    Literal,
}

impl fmt::Display for GitBinaryDiffMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GitBinaryDiffMethod::Delta => write!(f, "delta"),
            GitBinaryDiffMethod::Literal => write!(f, "literal"),
        }
    }
}

impl FromStr for GitBinaryDiffMethod {
    type Err = DiffParseError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "delta" => Ok(GitBinaryDiffMethod::Delta),
            "literal" => Ok(GitBinaryDiffMethod::Literal),
            _ => Err(DiffParseError::UnexpectedInput(
                DiffFormat::GitBinary,
                format!("{string}: unknown method expected \"delta\" or \"literal\""),
            )),
        }
    }
}

#[derive(Debug)]
pub struct GitBinaryDiffData {
    lines: Lines,
    method: GitBinaryDiffMethod,
    len_raw: usize,
    data_zipped: Vec<u8>,
}

impl GitBinaryDiffData {
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn iter(&self) -> Iter<'_, Line> {
        self.lines.iter()
    }

    pub fn get_raw_data(&self) -> DiffParseResult<Vec<u8>> {
        let data = inflate::inflate_bytes_zlib(&self.data_zipped)
            .map_err(DiffParseError::ZLibInflateError)?;
        if data.len() != self.len_raw {
            let msg = format!(
                "Inflated size {} doesn not match expected size {}",
                data.len(),
                self.len_raw
            );
            return Err(DiffParseError::ZLibInflateError(msg));
        }
        Ok(data)
    }

    pub fn apply_delta(&self, data: &[u8]) -> DiffParseResult<Vec<u8>> {
        let delta: Vec<u8> = match self.method {
            GitBinaryDiffMethod::Delta => self.get_raw_data()?,
            GitBinaryDiffMethod::Literal => {
                panic!("allempt to use \"literal\" data as a \"delta\"")
            }
        };
        git_delta::patch_delta(data, &delta).map_err(DiffParseError::GitDeltaError)
    }
}

#[derive(Debug)]
pub struct GitBinaryDiff {
    lines: Lines,
    forward: GitBinaryDiffData,
    reverse: GitBinaryDiffData,
}

impl GitBinaryDiff {
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn iter(&self) -> Iter<'_, Line> {
        self.lines.iter()
    }

    pub fn apply_to_contents<R, W>(
        &mut self,
        reader: &mut R,
        reverse: bool,
    ) -> DiffParseResult<Vec<u8>>
    where
        R: io::Read,
        W: io::Write,
    {
        if reverse {
            match self.reverse.method {
                GitBinaryDiffMethod::Delta => {
                    let mut data: Vec<u8> = Vec::new();
                    reader
                        .read_to_end(&mut data)
                        .map_err(DiffParseError::IOError)?;
                    self.reverse.apply_delta(&data)
                }
                GitBinaryDiffMethod::Literal => self.reverse.get_raw_data(),
            }
        } else {
            match self.forward.method {
                GitBinaryDiffMethod::Delta => {
                    let mut data: Vec<u8> = Vec::new();
                    reader
                        .read_to_end(&mut data)
                        .map_err(DiffParseError::IOError)?;
                    self.forward.apply_delta(&data)
                }
                GitBinaryDiffMethod::Literal => self.forward.get_raw_data(),
            }
        }
    }
}

pub struct GitBinaryDiffParser {
    start_cre: Regex,
    data_start_cre: Regex,
    blank_line_cre: Regex,
    data_line_cre: Regex,
}

impl Default for GitBinaryDiffParser {
    fn default() -> Self {
        Self::new()
    }
}

impl GitBinaryDiffParser {
    pub fn new() -> GitBinaryDiffParser {
        GitBinaryDiffParser {
            start_cre: Regex::new(r"^GIT binary patch(\n)?$").unwrap(),
            data_start_cre: Regex::new(r"^(literal|delta) (\d+)(\n)?$").unwrap(),
            blank_line_cre: Regex::new(r"^\s*(\n)?$").unwrap(),
            data_line_cre: Regex::new(r"^([a-zA-Z])([0-9a-zA-Z!#$%&()*+;<=>?@^_`{|}~-]+)(\n)?$")
                .unwrap(),
        }
    }

    // return lines consumed in due to possible swallowing of blank line making len() unreliable for advancing index
    fn get_data_at(
        &self,
        lines: &[Line],
        start_index: usize,
    ) -> DiffParseResult<(GitBinaryDiffData, usize)> {
        let captures = if let Some(captures) = self.data_start_cre.captures(&lines[start_index]) {
            captures
        } else {
            return Err(DiffParseError::SyntaxError(
                DiffFormat::GitBinary,
                start_index + 1,
            ));
        };
        let method = GitBinaryDiffMethod::from_str(captures.get(1).unwrap().as_str())?;
        let len_raw = usize::from_str(captures.get(2).unwrap().as_str())
            .map_err(|e| DiffParseError::ParseNumberError(e, start_index + 1))?;
        let mut index = start_index + 1;
        while index < lines.len() && self.data_line_cre.is_match(&lines[index]) {
            index += 1;
        }
        let end_data = index;
        // absorb the blank line if there is one
        if index < lines.len() && self.blank_line_cre.is_match(&lines[index]) {
            index += 1;
        }
        let data_zipped = git_base85::decode_lines(&lines[start_index + 1..end_data])?;
        Ok((
            GitBinaryDiffData {
                lines: lines[start_index..end_data].to_vec(),
                method,
                len_raw,
                data_zipped,
            },
            index - start_index,
        ))
    }

    pub fn get_diff_at(
        &self,
        lines: &[Line],
        start_index: usize,
    ) -> DiffParseResult<Option<GitBinaryDiff>> {
        if start_index >= lines.len() || !self.start_cre.is_match(&lines[start_index]) {
            return Ok(None);
        }
        let mut index = start_index + 1;
        let (forward, lines_consumed) = self.get_data_at(lines, index)?;
        index += lines_consumed;
        let (reverse, lines_consumed) = self.get_data_at(lines, index)?;
        index += lines_consumed;
        Ok(Some(GitBinaryDiff {
            lines: lines[start_index..index].to_vec(),
            forward,
            reverse,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lines::{Lines, LinesIfce};
    use std::path::Path;

    #[test]
    fn get_git_binary_diff_at_works() {
        let lines = Lines::read_from(Path::new("../test_diffs/test_2.binary_diff")).unwrap();
        let parser = GitBinaryDiffParser::new();
        let result = parser.get_diff_at(&lines, 1);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        for start_index in &[2, 12, 21, 30, 39, 49] {
            let result = parser.get_diff_at(&lines, *start_index);
            assert!(result.is_ok());
            let result = result.unwrap();
            assert!(result.is_some());
            let diff = result.unwrap();
            assert!(diff.iter().count() == diff.len());
            assert!(diff.forward.get_raw_data().is_ok());
            assert!(diff.reverse.get_raw_data().is_ok());
        }
    }
}

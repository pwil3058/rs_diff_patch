use regex::{Captures, Regex};
use std::option;
use std::slice::Iter;
use std::str::FromStr;

use pw_diff_lib::{Data, DataIfce};

use crate::text_diff::{
    extract_source_lines, DiffParseError, DiffParseResult, PathAndTimestamp, StartAndLength,
    StartsAndLengths, TextDiff, TextDiffHunk, TextDiffParser,
};
use crate::{DiffFormat, ALT_TIMESTAMP_RE_STR, EITHER_TIME_STAMP_RE_STR, PATH_RE_STR};

lazy_static::lazy_static! {
    pub static ref BEFORE_PATH_REGEX: Regex =
        Regex::new(&format!(r"^--- ({PATH_RE_STR})(\s+{EITHER_TIME_STAMP_RE_STR})?(.*)(\n)?$")).unwrap();

    pub static ref AFTER_PATH_REGEX: Regex =
        Regex::new(&format!(r"^\+\+\+ ({PATH_RE_STR})(\s+{EITHER_TIME_STAMP_RE_STR})?(.*)(\n)?$")).unwrap();

    pub static ref CHUNK_HEADER_REGEX: Regex =
        Regex::new(r"^@@\s+-(\d+)(,(\d+))?\s+\+(\d+)(,(\d+))?\s+@@\s*(.*)(\n)?$").unwrap();
}

fn path_and_time_stamp_from_captures(captures: &Captures) -> PathAndTimestamp {
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

pub fn before_path_and_time_stamp(line: &str) -> Option<PathAndTimestamp> {
    let captures = BEFORE_PATH_REGEX.captures(line)?;
    Some(path_and_time_stamp_from_captures(&captures))
}

pub fn after_path_and_time_stamp(line: &str) -> Option<PathAndTimestamp> {
    let captures = AFTER_PATH_REGEX.captures(line)?;
    Some(path_and_time_stamp_from_captures(&captures))
}

fn start_and_length_from_captures(
    captures: &Captures,
    line_num: usize,
    length: usize,
    line_number: usize,
) -> DiffParseResult<StartAndLength> {
    let start: usize = if let Some(m) = captures.get(line_num) {
        usize::from_str(m.as_str()).map_err(|e| DiffParseError::ParseNumberError(e, line_number))?
    } else {
        return Err(DiffParseError::SyntaxError(
            DiffFormat::Unified,
            line_number,
        ));
    };
    let length: usize = if let Some(m) = captures.get(length) {
        usize::from_str(m.as_str()).map_err(|e| DiffParseError::ParseNumberError(e, line_number))?
    } else {
        1
    };
    Ok(StartAndLength { start, length })
}

pub fn starts_and_lengths(
    line: &str,
    line_number: usize,
) -> DiffParseResult<Option<StartsAndLengths>> {
    match CHUNK_HEADER_REGEX.captures(line) {
        Some(captures) => {
            let before = start_and_length_from_captures(&captures, 1, 3, line_number)?;
            let after = start_and_length_from_captures(&captures, 4, 6, line_number)?;
            Ok(Some(StartsAndLengths { before, after }))
        }
        None => Ok(None),
    }
}

fn before_file_rec<'t>(line: &'t str) -> Option<Captures<'t>> {
    BEFORE_PATH_REGEX.captures(line)
}

fn after_file_rec<'t>(line: &'t str) -> Option<Captures<'t>> {
    AFTER_PATH_REGEX.captures(line)
}

pub struct UnifiedDiffChunk {
    pub lines: Vec<String>,
    pub ante_chunk: Box<UnifiedDiffStartAndLength>,
    pub post_chunk: Box<UnifiedDiffStartAndLength>,
}

pub type UnifiedDiff = TextDiff<UnifiedDiffChunk>;

trait HasTrailingWhiteSpace {
    fn has_trailing_white_space(&self) -> bool {
        false
    }
}

impl HasTrailingWhiteSpace for String {}

impl TextDiffHunk for UnifiedDiffChunk {
    fn len(&self) -> usize {
        self.lines.len()
    }

    fn iter(&self) -> Iter<String> {
        self.lines.iter()
    }

    fn ante_lines(&self) -> Vec<String> {
        extract_source_lines(&self.lines[1..], 1, |l| l.starts_with('+'))
    }

    fn post_lines(&self) -> Vec<String> {
        extract_source_lines(&self.lines[1..], 1, |l| l.starts_with('-'))
    }

    fn adds_trailing_white_space(&self) -> bool {
        for line in self.lines[1..].iter() {
            if line.starts_with('+') && line.has_trailing_white_space() {
                return true;
            }
        }
        false
    }

    // fn get_abstract_diff_hunk(&self) -> AbstractHunk {
    //     // NB: convert starting line numbers to 0 based indices
    //     // <https://www.gnu.org/software/diffutils/manual/html_node/Detailed-Unified.html#Detailed-Unified>
    //     // If a hunk contains just one line, only its start line number appears. Otherwise its line numbers
    //     // look like ‘start,count’. An empty hunk is considered to start at the line that follows the hunk.
    //     //
    //     // If a hunk and its context contain two or more lines, its line numbers look like ‘start,count’.
    //     // Otherwise only its end line number appears. An empty hunk is considered to end at the line that
    //     // precedes the hunk.
    //
    //     let ante_lines = self.ante_lines();
    //     let post_lines = self.post_lines();
    //     let ante_chunk = AbstractChunk {
    //         start_index: if !ante_lines.is_empty() {
    //             self.ante_chunk.start_line_num - 1
    //         } else {
    //             self.ante_chunk.start_line_num
    //         },
    //         lines: ante_lines,
    //     };
    //     let post_chunk = AbstractChunk {
    //         start_index: self.post_chunk.start_line_num - 1,
    //         lines: post_lines,
    //     };
    //     AbstractHunk::new(ante_chunk, post_chunk)
    // }
}

#[derive(Debug, Clone, Copy)]
pub struct UnifiedDiffStartAndLength {
    start_line_num: usize,
    length: usize,
}

impl UnifiedDiffStartAndLength {
    fn from_captures(
        captures: &Captures,
        line_num: usize,
        length: usize,
        line_number: usize,
    ) -> DiffParseResult<UnifiedDiffStartAndLength> {
        let start_line_num: usize = if let Some(m) = captures.get(line_num) {
            usize::from_str(m.as_str())
                .map_err(|e| DiffParseError::ParseNumberError(e, line_number))?
        } else {
            return Err(DiffParseError::SyntaxError(
                DiffFormat::Unified,
                line_number,
            ));
        };
        let length: usize = if let Some(m) = captures.get(length) {
            usize::from_str(m.as_str())
                .map_err(|e| DiffParseError::ParseNumberError(e, line_number))?
        } else {
            1
        };
        Ok(Self {
            start_line_num,
            length,
        })
    }
}

pub struct UnifiedDiffParser {
    ante_file_cre: Regex,
    post_file_cre: Regex,
    hunk_data_cre: Regex,
}

impl Default for UnifiedDiffParser {
    fn default() -> Self {
        Self::new()
    }
}

fn get_start_and_length_at<'a>(
    iter: &'a mut impl Iterator<Item = &'a String>,
) -> DiffParseResult<Option<UnifiedDiffChunk>> {
    let captures = if let Some(line) = iter.next() {
        let captures = if let Some(captures) = UnifiedDiffParser::hunk_data_cre.captures(&line) {
            captures
        } else {
            return Ok(None);
        };
    };
}

// pub trait OptionResult<T> {
//     fn option_result(&self, option: Option<T>) -> Result<T, _> {
//         if let Some(value) = option {
//             Ok(value)
//         } else {
//             Ok(None)
//         }
//     }
// }
//
// impl<T> OptionResult<T> for Option<T> {}

impl TextDiffParser<UnifiedDiffChunk> for UnifiedDiffParser {
    fn new() -> Self {
        UnifiedDiffParser {
            ante_file_cre: before_path_regex,
            post_file_cre: after_path_regex,
            hunk_data_cre: chunk_header_regex,
        }
    }

    fn get_hunk_at(
        &self,
        lines: &Data<String>,
        start_index: usize,
    ) -> DiffParseResult<Option<UnifiedDiffChunk>> {
        let mut iter = lines.subsequence_from(start_index);
        let captures = if let Some(line) = iter.next() {
            let captures = if let Some(captures) = self.hunk_data_cre.captures(&line) {
                captures
            } else {
                return Ok(None);
            };
        };
        let x = match iter.next() {
            None => return Ok(None),
            Some(line) => {
                let captures = if let Some(captures) = self.hunk_data_cre.captures(&line) {
                    captures
                } else {
                    return Ok(None);
                };
            }
        };
        let ante_chunk = UnifiedDiffStartAndLength::from_captures(&captures, 1, 3, start_index)?;
        let post_chunk = UnifiedDiffStartAndLength::from_captures(&captures, 4, 6, start_index)?;
        let mut index = start_index + 1;
        let start_context_length = 0;
        let end_context_length = 0;
        let at_the_front = true;
        let mut ante_count = 0;
        let mut post_count = 0;
        while ante_count < ante_chunk.length || post_count < post_chunk.length {
            if index >= lines.len() {
                return Err(DiffParseError::UnexpectedEndOfInput);
            }
            if lines[index].starts_with('-') {
                ante_count += 1
            } else if lines[index].starts_with('+') {
                post_count += 1
            } else if lines[index].starts_with(' ') {
                ante_count += 1;
                post_count += 1
            } else if !lines[index].starts_with('\\') {
                return Err(DiffParseError::UnexpectedEndHunk(
                    DiffFormat::Unified,
                    index,
                ));
            }
            index += 1
        }
        if index < lines.len() && lines[index].starts_with('\\') {
            index += 1
        }
        let hunk = UnifiedDiffChunk {
            lines: lines[start_index..index].to_vec(),
            ante_chunk: Box::new(ante_chunk),
            post_chunk: Box::new(post_chunk),
        };
        Ok(Some(hunk))
    }
}

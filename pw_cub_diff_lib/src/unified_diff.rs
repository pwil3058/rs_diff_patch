use regex::{Captures, Regex};
use std::slice::Iter;
use std::str::FromStr;

use crate::text_diff::{
    extract_source_lines, DiffParseError, DiffParseResult, TextDiff, TextDiffHunk, TextDiffParser,
};
use crate::{DiffFormat, ALT_TIMESTAMP_RE_STR, PATH_RE_STR, TIMESTAMP_RE_STR};

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

impl TextDiffParser<UnifiedDiffChunk> for UnifiedDiffParser {
    fn new() -> Self {
        let e_ts_re_str = format!("({TIMESTAMP_RE_STR}|{ALT_TIMESTAMP_RE_STR})");

        let e = format!(r"^--- ({PATH_RE_STR})(\s+{e_ts_re_str})?(.*)(\n)?$");
        let ante_file_cre = Regex::new(&e).unwrap();

        let e = format!(r"^\+\+\+ ({PATH_RE_STR})(\s+{e_ts_re_str})?(.*)(\n)?$");
        let post_file_cre = Regex::new(&e).unwrap();

        let hunk_data_cre =
            Regex::new(r"^@@\s+-(\d+)(,(\d+))?\s+\+(\d+)(,(\d+))?\s+@@\s*(.*)(\n)?$").unwrap();

        UnifiedDiffParser {
            ante_file_cre,
            post_file_cre,
            hunk_data_cre,
        }
    }

    fn ante_file_rec<'t>(&self, line: &'t str) -> Option<Captures<'t>> {
        self.ante_file_cre.captures(line)
    }

    fn post_file_rec<'t>(&self, line: &'t str) -> Option<Captures<'t>> {
        self.post_file_cre.captures(line)
    }

    fn get_hunk_at(
        &self,
        lines: &[String],
        start_index: usize,
    ) -> DiffParseResult<Option<UnifiedDiffChunk>> {
        let captures = if let Some(captures) = self.hunk_data_cre.captures(&lines[start_index]) {
            captures
        } else {
            return Ok(None);
        };
        let ante_chunk = UnifiedDiffStartAndLength::from_captures(&captures, 1, 3, start_index)?;
        let post_chunk = UnifiedDiffStartAndLength::from_captures(&captures, 4, 6, start_index)?;
        let mut index = start_index + 1;
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

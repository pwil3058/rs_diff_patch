use regex::{Captures, Regex};
use std::fmt::{Display, Formatter};
use std::io;
use std::str::FromStr;

use pw_diff_lib::range::Range;
use pw_diff_lib::{ApplyChunkFuzzyBasics, Data, DataIfce};

use crate::text_diff::{
    CheckEndOfInput, DiffParseError, DiffParseResult, PathAndTimestamp, StartAndLength,
};
use crate::{ALT_TIMESTAMP_RE_STR, PATH_RE_STR, TIMESTAMP_RE_STR};

lazy_static::lazy_static! {
    pub static ref EITHER_TIME_STAMP_RE_STR: String = format!("({TIMESTAMP_RE_STR}|{ALT_TIMESTAMP_RE_STR})");
    pub static ref BEFORE_PATH_REGEX: Regex =
        Regex::new(&format!(r"^--- ({PATH_RE_STR})\s+({TIMESTAMP_RE_STR}|{ALT_TIMESTAMP_RE_STR})?(.*)(\n)?$")).unwrap();

    pub static ref AFTER_PATH_REGEX: Regex =
        Regex::new(&format!(r"^\+\+\+ ({PATH_RE_STR})\s+({TIMESTAMP_RE_STR}|{ALT_TIMESTAMP_RE_STR})?(.*)(\n)?$")).unwrap();

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
        return Err(DiffParseError::SyntaxError(line_number));
    };
    let length: usize = if let Some(m) = captures.get(length) {
        usize::from_str(m.as_str()).map_err(|e| DiffParseError::ParseNumberError(e, line_number))?
    } else {
        1
    };
    Ok(StartAndLength { start, length })
}

#[derive(Debug, PartialEq, Clone)]
pub struct StartsAndLengths {
    pub before: StartAndLength,
    pub after: StartAndLength,
}

impl Display for StartsAndLengths {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.before.length == 1 {
            if self.after.length == 1 {
                write!(f, "@@ -{} +{} @@", self.before.start, self.after.start)
            } else {
                write!(
                    f,
                    "@@ -{} +{},{} @@",
                    self.before.start, self.after.start, self.after.length
                )
            }
        } else if self.after.length == 1 {
            write!(
                f,
                "@@ -{},{} +{} @@",
                self.before.start, self.before.length, self.after.start
            )
        } else {
            write!(
                f,
                "@@ -{},{} +{},{} @@",
                self.before.start, self.before.length, self.after.start, self.after.length
            )
        }
    }
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

pub struct UnifiedDiffChunk {
    pub starts_and_lengths: StartsAndLengths,
    pub before_lines: Box<[String]>,
    pub after_lines: Box<[String]>,
    pub context_lengths: (u8, u8),
    pub lines_consumed: usize,
}

impl UnifiedDiffChunk {
    pub fn get_from_at(lines: &Data<String>, start_index: usize) -> DiffParseResult<Option<Self>> {
        let mut iter = lines.subsequence_from(start_index);
        let line = match iter.next() {
            Some(line) => line,
            None => return Ok(None),
        };
        let starts_and_lengths = match starts_and_lengths(line, start_index)? {
            Some(sal) => sal,
            None => return Ok(None),
        };
        let mut start_context_length = 0u8;
        let mut end_context_length = 0u8;
        let mut at_the_front = true;
        let mut before_lines = vec![];
        let mut after_lines = vec![];
        let mut index = 0usize;
        let mut last_line_type = "X";
        while before_lines.len() < starts_and_lengths.before.length
            || after_lines.len() < starts_and_lengths.after.length
        {
            let line = *iter.next().check_end_of_input()?;
            if line.starts_with('-') {
                before_lines.push(line.as_str().to_string());
                end_context_length = 0;
                at_the_front = false;
                last_line_type = "-";
            } else if line.starts_with('+') {
                after_lines.push(line.as_str().to_string());
                end_context_length = 0;
                at_the_front = false;
                last_line_type = "+";
            } else if line.starts_with(' ') {
                before_lines.push(line.as_str().to_string());
                after_lines.push(line.as_str().to_string());
                if at_the_front {
                    start_context_length += 1
                } else {
                    end_context_length += 1
                }
                last_line_type = "-";
            } else {
                return Err(DiffParseError::UnexpectedEndChunk(start_index + index + 1));
            }
            index += 1;
        }
        let mut lines_consumed = index + 1;
        if let Some(line) = iter.next() {
            if line.starts_with("\\") {
                lines_consumed += 1;
                if last_line_type == "-" {
                    let line = before_lines.pop().unwrap();
                    before_lines.push(line.trim_end().to_string())
                } else if last_line_type == "+" {
                    let line = after_lines.pop().unwrap();
                    after_lines.push(line.trim_end().to_string())
                } else {
                    let line = before_lines.pop().unwrap();
                    before_lines.push(line.trim_end().to_string());
                    let line = after_lines.pop().unwrap();
                    after_lines.push(line.trim_end().to_string())
                }
            }
        }
        Ok(Some(Self {
            lines_consumed,
            before_lines: before_lines.into_boxed_slice(),
            after_lines: after_lines.into_boxed_slice(),
            starts_and_lengths,
            context_lengths: (start_context_length, end_context_length),
        }))
    }
}

impl ApplyChunkFuzzyBasics for UnifiedDiffChunk {
    fn context_lengths(&self) -> (u8, u8) {
        self.context_lengths
    }

    fn before_start(&self, reverse: bool) -> usize {
        if reverse {
            self.starts_and_lengths.after.start
        } else {
            self.starts_and_lengths.before.start
        }
    }

    fn before_length(&self, reverse: bool) -> usize {
        if reverse {
            self.starts_and_lengths.after.length
        } else {
            self.starts_and_lengths.before.length
        }
    }

    fn before_lines(&self, range: Option<Range>, reverse: bool) -> impl Iterator<Item = &String> {
        if let Some(range) = range {
            if reverse {
                self.after_lines[range.start()..range.end()].iter()
            } else {
                self.before_lines[range.start()..range.end()].iter()
            }
        } else {
            if reverse {
                self.after_lines.iter()
            } else {
                self.before_lines.iter()
            }
        }
    }

    fn before_write_into<W: io::Write>(
        &self,
        into: &mut W,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        if let Some(reductions) = reductions {
            let range = Range(
                reductions.0 as usize,
                self.before_length(reverse) - reductions.1 as usize,
            );
            for line in self.before_lines(Some(range), reverse) {
                into.write_all(line.as_bytes())?;
            }
        } else {
            for line in self.before_lines(None, reverse) {
                into.write_all(line.as_bytes())?;
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::unified_diff::UnifiedDiffChunk;
    use pw_diff_lib::data::Data;
    use std::fs::File;

    static UNIFIED_DIFF_CHUNK: &str = "--- lao	2002-02-21 23:30:39.942229878 -0800
+++ tzu	2002-02-21 23:30:50.442260588 -0800
@@ -1,7 +1,6 @@
-The Way that can be told of is not the eternal Way;
-The name that can be named is not the eternal name.
 The Nameless is the origin of Heaven and Earth;
-The Named is the mother of all things.
+The named is the mother of all things.
+
 Therefore let there always be non-being,
   so we may see their subtlety,
 And let there always be being,
@@ -9,3 +8,6 @@
 The two are the same,
 But after they are produced,
   they have different names.
+They both may be called deep and profound.
+Deeper and more profound,
+The door of all subtleties!
";

    #[test]
    fn unified_diff_chunk_parse_string() {
        let diff_lines = Data::<String>::from(UNIFIED_DIFF_CHUNK);
        assert!(UnifiedDiffChunk::get_from_at(&diff_lines, 2).is_ok());
        assert!(UnifiedDiffChunk::get_from_at(&diff_lines, 2)
            .unwrap()
            .is_some());
        assert!(UnifiedDiffChunk::get_from_at(&diff_lines, 1)
            .unwrap()
            .is_none());
    }

    #[test]
    fn unified_diff_chunk_parse_from_file() {
        let file = File::open("test_diffs/test_1.diff").unwrap();
        let lines = Data::<String>::read(file).unwrap();
        let result = UnifiedDiffChunk::get_from_at(&lines, 0);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let result = UnifiedDiffChunk::get_from_at(&lines, 14);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_none());
        // let diff = result.unwrap();
        // assert!(diff.lines_consumed == diff.len());
    }
}

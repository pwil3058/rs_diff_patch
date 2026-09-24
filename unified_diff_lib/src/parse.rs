// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use regex::{Captures, Regex};
use std::num::ParseIntError;
use std::str::FromStr;
use std::sync::LazyLock;

use thiserror::Error;

use longest_common_subsequence::range::Range;
use longest_common_subsequence::sequence::Seq;
use pw_diff_lib::apply_text::{ApplyClumpFuzzy, ApplyClumpsFuzzy, TextClumpBasics};
use pw_diff_lib::changes::ChangeBasics;

use crate::{PathAndTimestamp, StartAndLength, StartsAndLengths};

pub const TIMESTAMP_RE_STR: &str = r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(\.\d{9})? [-+]{1}\d{4}";
pub const ALT_TIMESTAMP_RE_STR: &str =
    r"[A-Z][a-z]{2} [A-Z][a-z]{2} \d{2} \d{2}:\d{2}:\d{2} \d{4} [-+]{1}\d{4}";
pub const PATH_RE_STR: &str = r###""([^"]+)"|(\S+)"###;

#[derive(Debug, Error)]
pub enum DiffParseError {
    #[error("{0} at line {1}")]
    ParseNumberError(ParseIntError, usize),
    #[error("Unexpected end of input")]
    UnexpectedEndOfInput,
    #[error("Unexpected end of clump at line {0}")]
    UnexpectedEndClump(usize),
    #[error("Syntax error at line {0}")]
    SyntaxError(usize),
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

pub static BEFORE_PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^--- ({PATH_RE_STR})\s+({TIMESTAMP_RE_STR}|{ALT_TIMESTAMP_RE_STR})?(.*)(\n)?$"
    ))
    .unwrap()
});

pub static AFTER_PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^\+\+\+ ({PATH_RE_STR})\s+({TIMESTAMP_RE_STR}|{ALT_TIMESTAMP_RE_STR})?(.*)(\n)?$"
    ))
    .unwrap()
});

pub static CLUMP_HEADER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^@@\s+-(\d+)(,(\d+))?\s+\+(\d+)(,(\d+))?\s+@@\s*(.*)(\n)?$").unwrap()
});

fn path_and_time_stamp_from_captures(captures: &Captures) -> PathAndTimestamp {
    let file_path = if let Some(path) = captures.get(2) {
        path.as_str()
    } else {
        captures.get(3).unwrap().as_str() // TODO: confirm unwrap is OK here
    };
    let time_stamp = captures.get(4).map(|ts| ts.as_str().to_string());
    PathAndTimestamp {
        file_path: file_path.to_string().into(),
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

pub fn starts_and_lengths(
    line: &str,
    line_number: usize,
) -> DiffParseResult<Option<StartsAndLengths>> {
    match CLUMP_HEADER_REGEX.captures(line) {
        Some(captures) => {
            let before = start_and_length_from_captures(&captures, 1, 3, line_number)?;
            let after = start_and_length_from_captures(&captures, 4, 6, line_number)?;
            Ok(Some(StartsAndLengths { before, after }))
        }
        None => Ok(None),
    }
}

pub struct UnifiedDiffClump {
    pub starts_and_lengths: StartsAndLengths,
    pub before_lines: Box<[String]>,
    pub after_lines: Box<[String]>,
    pub context_lengths: (u8, u8),
    pub lines_consumed: usize,
}

impl UnifiedDiffClump {
    pub fn get_from_at(lines: &Seq<String>, start_index: usize) -> DiffParseResult<Option<Self>> {
        let mut iter = lines.subsequence(lines.range_from(start_index));
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
                return Err(DiffParseError::UnexpectedEndClump(start_index + index + 1));
            }
            index += 1;
        }
        let mut lines_consumed = index + 1;
        if let Some(line) = iter.next()
            && line.starts_with("\\")
        {
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
        Ok(Some(Self {
            lines_consumed,
            before_lines: before_lines.into_boxed_slice(),
            after_lines: after_lines.into_boxed_slice(),
            starts_and_lengths,
            context_lengths: (start_context_length, end_context_length),
        }))
    }
}

impl ChangeBasics for UnifiedDiffClump {
    fn before_start(&self, reverse: bool) -> usize {
        if reverse {
            self.starts_and_lengths.after.start
        } else {
            self.starts_and_lengths.before.start
        }
    }

    fn before_end(&self, reverse: bool) -> usize {
        self.before_start(reverse) + self.before_length(reverse)
    }

    fn before_length(&self, reverse: bool) -> usize {
        if reverse {
            self.starts_and_lengths.after.length
        } else {
            self.starts_and_lengths.before.length
        }
    }
}

impl ApplyClumpFuzzy for UnifiedDiffClump {}

impl TextClumpBasics for UnifiedDiffClump {
    fn context_lengths(&self) -> (u8, u8) {
        self.context_lengths
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
}

pub struct UnifiedDiffClumps(pub Box<[UnifiedDiffClump]>);

impl UnifiedDiffClumps {
    pub fn get_from_at(lines: &Seq<String>, start_index: usize) -> DiffParseResult<Self> {
        let mut clumps = vec![];
        let mut index = start_index;
        while let Some(clump) = UnifiedDiffClump::get_from_at(lines, index)? {
            index += clump.lines_consumed;
            clumps.push(clump);
        }
        Ok(Self(clumps.into_boxed_slice()))
    }
}

impl ApplyClumpsFuzzy<UnifiedDiffClump> for UnifiedDiffClumps {
    fn clumps<'s>(&'s self) -> impl Iterator<Item = &'s UnifiedDiffClump>
    where
        UnifiedDiffClump: 's,
    {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use longest_common_subsequence::sequence::Seq;
    use pw_diff_lib::sequence::*;
    use std::fs::File;

    use crate::parse::{UnifiedDiffClump, UnifiedDiffClumps};

    static UNIFIED_DIFF_CLUMP: &str = "--- lao	2002-02-21 23:30:39.942229878 -0800
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
    fn unified_diff_clump_parse_string() {
        let diff_lines = Seq::<String>::from_iter(
            UNIFIED_DIFF_CLUMP
                .split_inclusive('\n')
                .map(|s| s.to_string()),
        );
        assert!(UnifiedDiffClump::get_from_at(&diff_lines, 2).is_ok());
        assert!(
            UnifiedDiffClump::get_from_at(&diff_lines, 2)
                .unwrap()
                .is_some()
        );
        assert!(
            UnifiedDiffClump::get_from_at(&diff_lines, 1)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn unified_diff_clump_parse_from_file() {
        let file = File::open("test_diffs/test_1.diff").unwrap();
        let lines = Seq::<String>::read(file).unwrap();
        let result = UnifiedDiffClump::get_from_at(&lines, 0);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let result = UnifiedDiffClump::get_from_at(&lines, 14);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn unified_diff_clumps_parse_string() {
        let diff_lines = Seq::<String>::from_iter(
            UNIFIED_DIFF_CLUMP
                .split_inclusive('\n')
                .map(|s| s.to_string()),
        );
        let clumps = UnifiedDiffClumps::get_from_at(&diff_lines, 2).unwrap();
        assert_eq!(clumps.0.len(), 2);
    }

    #[test]
    fn unified_diff_clumps_parse_from_file() {
        let file = File::open("test_diffs/test_1.diff").unwrap();
        let lines = Seq::<String>::read(file).unwrap();
        let result = UnifiedDiffClumps::get_from_at(&lines, 0);
        assert!(result.is_ok());
    }
}

// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use crate::{PathAndTimestamp, StartAndLength, StartsAndLengths, extract_timestamp};
use longest_common_subsequence::changes::{Change, ChangeClumpIter, Changes};
use longest_common_subsequence::{range::Len, sequence::Seq};
use pw_diff_lib::sequence::ReadSequence;
use std::fmt::Display;
use std::fs::File;
use std::io;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;

pub struct UnifiedClump {
    pub header: String,
    pub lines: Vec<String>,
}

impl UnifiedClump {
    pub fn write_into<W: Write>(&self, into: &mut W) -> io::Result<()> {
        into.write_all(self.header.as_bytes())?;
        for line in self.lines.iter() {
            into.write_all(line.as_bytes())?;
        }
        Ok(())
    }
}

impl Display for UnifiedClump {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut string = self.header.clone();
        for line in self.lines.iter() {
            string.push_str(line)
        }
        write!(formatter, "{}", string)
    }
}

pub struct UnifiedClumpIter<'a> {
    pub before: &'a Seq<String>,
    pub after: &'a Seq<String>,
    pub iter: ChangeClumpIter<'a, String>,
}

impl<'a> Iterator for UnifiedClumpIter<'a> {
    type Item = UnifiedClump;

    fn next(&mut self) -> Option<Self::Item> {
        let change_clump = self.iter.next()?;

        let starts = change_clump.starts();
        let ends = change_clump.ends();
        let before_start_length = StartAndLength {
            start: starts.0,
            length: ends.0 - starts.0,
        };
        let after_start_length = StartAndLength {
            start: starts.1,
            length: ends.1 - starts.1,
        };
        let starts_and_lengths = StartsAndLengths {
            before: before_start_length,
            after: after_start_length,
        };
        let header = format!("{starts_and_lengths}");

        let mut lines = vec![];
        for change in change_clump.iter() {
            use Change::*;
            match change {
                NoChange(common_subsequence) => {
                    for line in self.before.subsequence(common_subsequence.left_range()) {
                        lines.push(format!(" {line}"));
                    }
                }
                Delete(before_range, _) => {
                    for line in self.before.subsequence(*before_range) {
                        lines.push(format!("-{line}"));
                    }
                }
                Insert(_, after_range) => {
                    for line in self.after.subsequence(*after_range) {
                        lines.push(format!("+{line}"));
                    }
                }
                Replace(before_range, after_range) => {
                    for line in self.before.subsequence(*before_range) {
                        lines.push(format!("-{line}"));
                    }
                    for line in self.after.subsequence(*after_range) {
                        lines.push(format!("+{line}"));
                    }
                }
            }
        }

        if let Some(last_line) = lines.last() {
            if !last_line.ends_with('\n') {
                lines.push("\n\\\n".to_string());
            }
        }

        Some(UnifiedClump { header, lines })
    }
}

pub struct UnifiedClumps(pub Box<[UnifiedClump]>);

impl UnifiedClumps {
    pub fn new(before: &Seq<String>, after: &Seq<String>, context: u8) -> Self {
        let changes = Changes::new(before, after);
        let iter = UnifiedClumpIter {
            before,
            after,
            iter: changes.change_clumps(context),
        };
        Self(iter.collect())
    }

    pub fn write_into<W: Write>(&self, into: &mut W) -> io::Result<()> {
        for clump in self.0.iter() {
            clump.write_into(into)?;
        }
        Ok(())
    }
}

impl Deref for UnifiedClumps {
    type Target = Box<[UnifiedClump]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for UnifiedClumps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut string = "".to_string();
        for clump in self.0.iter() {
            string.push_str(format!("{clump}").as_str());
        }
        write!(formatter, "{}", string)
    }
}

pub struct UnifiedDiff {
    pub before: PathAndTimestamp,
    pub after: PathAndTimestamp,
    pub unified_clumps: UnifiedClumps,
}

impl UnifiedDiff {
    pub fn new(before: impl AsRef<Path>, after: impl AsRef<Path>, context: u8) -> io::Result<Self> {
        let before_path = before.as_ref().to_owned();
        let after_path = after.as_ref().to_owned();

        let before_timestamp = extract_timestamp(&before_path);
        let after_timestamp = extract_timestamp(&after_path);

        let before_lines = Seq::<String>::read(File::open(&before)?)?;
        let after_lines = Seq::<String>::read(File::open(&after)?)?;
        let unified_clumps = UnifiedClumps::new(&before_lines, &after_lines, context);

        Ok(Self {
            before: PathAndTimestamp {
                file_path: before_path,
                time_stamp: before_timestamp,
            },
            after: PathAndTimestamp {
                file_path: after_path,
                time_stamp: after_timestamp,
            },
            unified_clumps,
        })
    }

    pub fn write_into<W: Write>(&self, into: &mut W) -> io::Result<()> {
        let string = format!("--- {}", self.before.file_path.to_string_lossy());
        into.write_all(string.as_bytes())?;
        if let Some(time_stamp) = self.before.time_stamp.as_ref() {
            into.write_all(time_stamp.as_bytes())?;
        }
        into.write_all("\n".as_bytes())?;

        let string = format!("+++ {}", self.after.file_path.to_string_lossy());
        into.write_all(string.as_bytes())?;
        if let Some(time_stamp) = self.after.time_stamp.as_ref() {
            into.write_all(time_stamp.as_bytes())?;
        }
        into.write_all("\n".as_bytes())?;

        self.unified_clumps.write_into(into)?;
        Ok(())
    }
}

impl Display for UnifiedDiff {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut buffer = Vec::new();
        self.write_into(&mut buffer).unwrap();
        write!(formatter, "{}", String::from_utf8(buffer).unwrap())
    }
}

#[cfg(test)]
mod generated_unified_diff_tests {
    use super::*;

    fn line_seq(text: &str) -> Seq<String> {
        Seq::from_iter(text.split_inclusive('\n').map(|s| s.to_string()))
    }

    #[test]
    fn generate() {
        let before_lines = line_seq("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
        let after_lines = line_seq("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
        let clumps = UnifiedClumps::new(&before_lines, &after_lines, 2);
        assert_eq!(
            format!("{clumps}"),
            "@@ -0,8 +0,7 @@ A\n-B\n C\n D\n+Ef\n+Fg\n-E\n-F\n G\n H\n@@ -9,4 +8,5 @@ J\n K\n+H\n L\n M\n"
        );
        let mut buffer = Vec::new();
        clumps.write_into(&mut buffer).unwrap();
        let string = String::from_utf8(buffer).unwrap();
        assert_eq!(
            &string,
            "@@ -0,8 +0,7 @@ A\n-B\n C\n D\n+Ef\n+Fg\n-E\n-F\n G\n H\n@@ -9,4 +8,5 @@ J\n K\n+H\n L\n M\n"
        );
    }

    #[test]
    fn generate_from_files() {
        let before_path_buf = Path::new("../test_files/file_1_original");
        let after_path_buf = Path::new("../test_files/file_1_modified");
        let unified_diff = UnifiedDiff::new(before_path_buf, after_path_buf, 2).unwrap();
        assert_eq!(
            unified_diff.to_string(),
            "--- ../test_files/file_1_original
+++ ../test_files/file_1_modified
@@ -9,6 +9,6 @@ Line 10 original
 Line 11 original
+Line 12 modified
+Line 13 modified
-Line 12 original
-Line 13 original
 Line 14 original
 Line 15 original
@@ -24,4 +24,5 @@ Line 25 original
 Line 26 original
+Line 26.1 inserted
 Line 27 original
 Line 28 original
@@ -61,3 +62,3 @@ Line 62 original
 Line 63 original
+Line 64 modified
-Line 64 original
"
        );
    }
}

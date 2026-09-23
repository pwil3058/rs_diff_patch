// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use crate::{StartAndLength, StartsAndLengths};
use longest_common_subsequence::{range::Len, sequence::Seq};
use pw_diff_lib::changes::{Change, ChangeClumpIter, Changes};
use std::fmt::Display;
use std::io;
use std::io::Write;
use std::ops::Deref;

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
        let before_start_and_end = StartAndLength {
            start: starts.0,
            length: ends.0 - starts.0,
        };
        let after_start_and_end = StartAndLength {
            start: starts.1,
            length: ends.1 - starts.1,
        };
        let starts_and_lengths = StartsAndLengths {
            before: before_start_and_end,
            after: after_start_and_end,
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
                    if before_range.len() < after_range.len() {
                        for line in self.before.subsequence(*before_range) {
                            lines.push(format!("-{line}"));
                        }
                        for line in self.after.subsequence(*after_range) {
                            lines.push(format!("+{line}"));
                        }
                    } else {
                        for line in self.after.subsequence(*after_range) {
                            lines.push(format!("+{line}"));
                        }
                        for line in self.before.subsequence(*before_range) {
                            lines.push(format!("-{line}"));
                        }
                    }
                }
            }
        }
        if !lines
            .last()
            .expect("impl Iterator for UnifiedClumpIter")
            .ends_with("\n")
        {
            lines.push("\n\\\n".to_string());
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
        Self(iter.collect::<Vec<_>>().into_boxed_slice())
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
    }
}

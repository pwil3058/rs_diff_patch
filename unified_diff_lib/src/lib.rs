// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

pub mod generate;
pub mod parse_and_apply;

use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};

#[derive(Debug, PartialEq, Clone)]
pub struct PathAndTimestamp {
    pub file_path: PathBuf,
    pub time_stamp: Option<String>,
}

/// Helper function to extract a file's last modified timestamp as a formatted string.
/// Returns a standard unified diff timestamp fragment preceded by a tab: "\tYYYY-MM-DD HH:MM:SS.fffffffff ±hhmm"
fn extract_timestamp(path: &Path) -> Option<String> {
    path.metadata().ok()?.modified().ok().map(|system_time| {
        let datetime: DateTime<Local> = system_time.into();
        // %F = YYYY-MM-DD, %T = HH:MM:SS, %.9f = nanoseconds, %z = timezone offset
        format!("\t{}", datetime.format("%F %T%.9f %z"))
    })
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

impl Display for StartsAndLengths {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.before.length == 1 {
            if self.after.length == 1 {
                write!(f, "@@ -{} +{} @@\n", self.before.start, self.after.start)
            } else {
                write!(
                    f,
                    "@@ -{} +{},{} @@\n",
                    self.before.start, self.after.start, self.after.length
                )
            }
        } else if self.after.length == 1 {
            write!(
                f,
                "@@ -{},{} +{} @@\n",
                self.before.start, self.before.length, self.after.start
            )
        } else {
            write!(
                f,
                "@@ -{},{} +{},{} @@\n",
                self.before.start, self.before.length, self.after.start, self.after.length
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{generate, parse_and_apply};
    use longest_common_subsequence::sequence::Seq;
    use pw_diff_lib::sequence::*;
    use std::fs::File;

    #[test]
    fn generate_and_apply() {
        let before_file_path = "../test_files/file_2_original";
        let after_file_path = "../test_files/file_2_modified";

        let _before_lines = Seq::<String>::read(File::open(before_file_path).unwrap()).unwrap();
        let _after_lines = Seq::<String>::read(File::open(after_file_path).unwrap()).unwrap();

        let generated_diff =
            generate::UnifiedDiff::new(before_file_path, after_file_path, 2).unwrap();
        let mut buffer = Vec::<u8>::new();
        generated_diff.write_into(&mut buffer).unwrap();
        let generated_diff_lines = Seq::<String>::read(buffer.as_slice()).unwrap();
        let _parsed_diff_clumps =
            parse_and_apply::UnifiedDiffClumps::get_from_at(&generated_diff_lines, 2).unwrap();
    }
}

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
        println!("DT {:?}", datetime);
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

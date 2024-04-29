// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::io::{self, BufRead, BufReader, Read};

use crate::range::*;

pub trait BasicLines: Len + Default {
    fn lines(&self, range: Range) -> impl DoubleEndedIterator<Item = &str>;
    fn range_from(&self, start: usize) -> Range {
        Range(start, self.len())
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Lines(pub Vec<String>);

impl Len for Lines {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl BasicLines for Lines {
    fn lines(&self, range: Range) -> impl DoubleEndedIterator<Item = &str> {
        self.0[range.0..range.1].iter().map(|s| s.as_str())
    }
}

impl Lines {
    pub fn read<R: Read>(read: R) -> io::Result<Lines> {
        let mut reader = BufReader::new(read);
        let mut lines = vec![];
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                break;
            } else {
                lines.push(line)
            }
        }
        Ok(Lines(lines))
    }
}

impl From<String> for Lines {
    fn from(text: String) -> Self {
        let eol = if let Some(_) = text.find("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        Self(text.split_inclusive(eol).map(|s| s.to_string()).collect())
    }
}

impl From<&str> for Lines {
    fn from(arg: &str) -> Self {
        Self::from(arg.to_string())
    }
}

#[cfg(test)]
pub mod test_lines {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn lines() {
        let lines: Lines = String::from("a\nb\nc\nd\n").into();
        assert_eq!(lines.len(), 4);
        assert_eq!(
            vec!["b\n", "c\n"],
            lines.lines(Range(1, 3)).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["b\n", "c\n", "d\n"],
            lines.lines(lines.range_from(1)).collect::<Vec<&str>>()
        );
    }

    #[test]
    fn read_lines() {
        let cursor = Cursor::new("A\nB\nC");
        let lines = Lines::read(cursor).unwrap();
        assert_eq!(
            Lines(vec!["A\n".to_string(), "B\n".to_string(), "C".to_string()]),
            lines
        );
        let cursor = Cursor::new("A\r\nB\r\nC");
        let lines = Lines::read(cursor).unwrap();
        assert_eq!(
            Lines(vec![
                "A\r\n".to_string(),
                "B\r\n".to_string(),
                "C".to_string()
            ]),
            lines
        );
    }
}

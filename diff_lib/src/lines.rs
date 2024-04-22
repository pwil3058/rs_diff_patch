// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::ops::RangeBounds;

use crypto_hash;

use crate::range::*;

pub trait MapKey {
    fn map_key(&self) -> Vec<u8>;
}

impl MapKey for str {
    fn map_key(&self) -> Vec<u8> {
        //self.as_bytes().to_vec()
        crypto_hash::digest(crypto_hash::Algorithm::SHA1, &self.as_bytes())
    }
}

pub trait BasicLines: Len + Default {
    fn has_final_eol(&self) -> bool;
    fn eol(&self) -> &str;
    fn lines(&self, range_bounds: impl RangeBounds<usize>) -> impl Iterator<Item = &str>;
    fn lines_reversed(&self, range_bounds: impl RangeBounds<usize>) -> impl Iterator<Item = &str>;
}

#[derive(Debug, Default)]
pub struct LazyLines {
    text: String,
    length: usize,
    eol: &'static str,
    has_final_eol: bool,
}

impl Len for LazyLines {
    fn len(&self) -> usize {
        self.length
    }
}

impl BasicLines for LazyLines {
    fn has_final_eol(&self) -> bool {
        self.has_final_eol
    }

    fn eol(&self) -> &str {
        self.eol
    }

    fn lines(&self, range_bounds: impl RangeBounds<usize>) -> impl Iterator<Item = &str> {
        let range = Range::from(range_bounds);
        self.text.lines().skip(range.start()).take(range.len())
    }

    fn lines_reversed(&self, range_bounds: impl RangeBounds<usize>) -> impl Iterator<Item = &str> {
        let range = Range::from(range_bounds);
        self.text
            .lines()
            .rev()
            .skip(self.length - range.end().min(self.length))
            .take(range.end().min(self.length) - range.start())
    }
}

impl From<String> for LazyLines {
    fn from(text: String) -> Self {
        let length = text.split_inclusive("\n").count();
        let has_final_eol = text.ends_with("\n");
        let eol = if let Some(_) = text.find("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        Self {
            text,
            length,
            has_final_eol,
            eol,
        }
    }
}

impl From<&str> for LazyLines {
    fn from(arg: &str) -> Self {
        Self::from(arg.to_string())
    }
}

#[cfg(test)]
pub mod test_lines {
    use super::*;

    #[test]
    fn lazy_lines() {
        let lazy_lines: LazyLines = String::from("a\nb\nc\nd\n").into();
        assert!(lazy_lines.has_final_eol());
        assert_eq!(lazy_lines.eol(), "\n");
        assert_eq!(lazy_lines.len(), 4);
        assert_eq!(
            vec!["b", "c"],
            lazy_lines.lines(1..3).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["b", "c", "d"],
            lazy_lines.lines(1..).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["c", "b"],
            lazy_lines.lines_reversed(1..3).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["d", "c", "b"],
            lazy_lines.lines_reversed(1..).collect::<Vec<&str>>()
        );
    }

    #[test]
    fn lazy_lines_no_final_eol() {
        let lazy_lines: LazyLines = String::from("a\nb\nc\nd").into();
        assert!(!lazy_lines.has_final_eol());
        assert_eq!(lazy_lines.eol(), "\n");
        assert_eq!(lazy_lines.len(), 4);
        assert_eq!(
            vec!["b", "c"],
            lazy_lines.lines(1..3).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["b", "c", "d"],
            lazy_lines.lines(1..).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["c", "b"],
            lazy_lines.lines_reversed(1..3).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["d", "c", "b"],
            lazy_lines.lines_reversed(1..).collect::<Vec<&str>>()
        );
    }

    #[test]
    fn lazy_lines_ms_eol() {
        let lazy_lines: LazyLines = String::from("a\r\nb\r\nc\r\nd\r\n").into();
        assert!(lazy_lines.has_final_eol());
        assert_eq!(lazy_lines.eol(), "\r\n");
        assert_eq!(lazy_lines.len(), 4);
        assert_eq!(
            vec!["b", "c"],
            lazy_lines.lines(1..3).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["b", "c", "d"],
            lazy_lines.lines(1..).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["c", "b"],
            lazy_lines.lines_reversed(1..3).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["d", "c", "b"],
            lazy_lines.lines_reversed(1..).collect::<Vec<&str>>()
        );
    }
}

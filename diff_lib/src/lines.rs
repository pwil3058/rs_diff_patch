// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::collections::Bound;
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
    fn lines(&self, range_bounds: impl RangeBounds<usize>)
        -> impl DoubleEndedIterator<Item = &str>;
    fn to_range(&self, bounds: impl RangeBounds<usize>) -> Range {
        let start = match bounds.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i + 1,
            _ => 0,
        };
        let end = match bounds.end_bound() {
            Bound::Included(i) => *i + 1,
            Bound::Excluded(i) => *i,
            _ => self.len(),
        };

        Range(start, end)
    }
}

#[derive(Debug, Default)]
pub struct Lines(pub Vec<String>);

impl Len for Lines {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl BasicLines for Lines {
    fn lines(
        &self,
        range_bounds: impl RangeBounds<usize>,
    ) -> impl DoubleEndedIterator<Item = &str> {
        let range = self.to_range(range_bounds);
        self.0[range.0..range.1].iter().map(|s| s.as_str())
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

    #[test]
    fn lazy_lines() {
        let lazy_lines: Lines = String::from("a\nb\nc\nd\n").into();
        assert_eq!(lazy_lines.len(), 4);
        assert_eq!(
            vec!["b\n", "c\n"],
            lazy_lines.lines(1..3).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["b\n", "c\n", "d\n"],
            lazy_lines.lines(1..).collect::<Vec<&str>>()
        );
    }
}

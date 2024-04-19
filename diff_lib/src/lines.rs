// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::collections::{Bound, HashMap};
use std::ops::RangeBounds;

use crypto_hash;

use crate::crange::*;

pub trait MapKey {
    fn map_key(&self) -> Vec<u8>;
}

impl MapKey for str {
    fn map_key(&self) -> Vec<u8> {
        //self.as_bytes().to_vec()
        crypto_hash::digest(crypto_hash::Algorithm::SHA1, &self.as_bytes())
    }
}

#[derive(Debug, Default)]
pub struct LineIndices(HashMap<Vec<u8>, Vec<usize>>);

impl LineIndices {
    fn add(&mut self, line: &str, index: usize) {
        let key = line.map_key();
        if let Some(vec) = self.0.get_mut(&key) {
            vec.push(index);
        } else {
            self.0.insert(key, vec![index]);
        }
    }

    pub fn get(&self, line: &str) -> Option<&Vec<usize>> {
        self.0.get(&line.map_key())
    }
}

pub trait BasicLines: Len + Default {
    fn lines(&self, range_bounds: impl RangeBounds<usize>) -> impl Iterator<Item = &str>;
    fn lines_reversed(&self, range_bounds: impl RangeBounds<usize>) -> impl Iterator<Item = &str>;
}

pub trait DiffInputLines: BasicLines {
    fn get_line_indices(&self) -> LineIndices {
        let mut line_indices = LineIndices::default();
        for (i, item) in self.lines(..).enumerate() {
            line_indices.add(item, i);
        }
        line_indices
    }
}

#[derive(Debug, Default)]
pub struct LazyLines {
    text: String,
    length: usize,
}

impl Len for LazyLines {
    fn len(&self) -> usize {
        self.length
    }
}

impl BasicLines for LazyLines {
    fn lines(&self, range_bounds: impl RangeBounds<usize>) -> impl Iterator<Item = &str> {
        let range = CRange::from(range_bounds);
        self.text
            .split_inclusive("\n")
            .skip(range.start())
            .take(range.len())
    }

    fn lines_reversed(&self, range_bounds: impl RangeBounds<usize>) -> impl Iterator<Item = &str> {
        let range = CRange::from(range_bounds);
        let iter = self.text.split_inclusive('\n').rev();
        iter.skip(self.length - range.end().min(self.length))
            .take(range.end().min(self.length) - range.start())
    }
}

impl DiffInputLines for LazyLines {}

impl From<String> for LazyLines {
    fn from(text: String) -> Self {
        let length = text.split_inclusive("\n").count();
        Self { text, length }
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
        assert_eq!(lazy_lines.len(), 4);
        assert_eq!(
            vec!["b\n", "c\n"],
            lazy_lines.lines(1..3).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["b\n", "c\n", "d\n"],
            lazy_lines.lines(1..).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["c\n", "b\n"],
            lazy_lines.lines_reversed(1..3).collect::<Vec<&str>>()
        );
        assert_eq!(
            vec!["d\n", "c\n", "b\n"],
            lazy_lines.lines_reversed(1..).collect::<Vec<&str>>()
        );
    }

    #[test]
    fn line_indices() {
        let lines = LazyLines::from("a\nb\nc\nd\na\nb\nc\nd\n");
        let indices = lines.get_line_indices();
        assert_eq!(indices.get("b\n"), Some(&vec![1usize, 5usize]));
        assert_eq!(indices.get("f\n"), None);
    }
}

// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::lines::{BasicLines, Lines};
use serde::{Deserialize, Serialize};
use std::collections::Bound;
use std::ops::RangeBounds;

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet {
    pub start: usize,
    pub lines: Vec<String>,
}

impl Snippet {
    pub fn length(&self, reductions: Option<(usize, usize)>) -> usize {
        if let Some((start_reduction, end_reduction)) = reductions {
            self.lines.len() - start_reduction - end_reduction
        } else {
            self.lines.len()
        }
    }

    pub fn start(&self, offset: isize, reductions: Option<(usize, usize)>) -> usize {
        if let Some(reductions) = reductions {
            reductions.0 + self.start.checked_add_signed(offset).expect("underflow")
        } else {
            self.start.checked_add_signed(offset).expect("underflow")
        }
    }

    pub fn lines(&self, reductions: Option<(usize, usize)>) -> impl Iterator<Item = &String> {
        if let Some((start_reduction, end_reduction)) = reductions {
            self.lines[start_reduction..self.lines.len() - end_reduction].iter()
        } else {
            self.lines.iter()
        }
    }

    pub fn lines_as_text(&self, reductions: Option<(usize, usize)>) -> String {
        if let Some((start_reduction, end_reduction)) = reductions {
            self.lines[start_reduction..self.lines.len() - end_reduction].join("")
        } else {
            self.lines.join("")
        }
    }
}

pub trait ExtractSnippet: BasicLines {
    fn extract_snippet(&self, range_bounds: impl RangeBounds<usize>) -> Snippet {
        let start = match range_bounds.start_bound() {
            Bound::Included(start) => *start,
            Bound::Excluded(start) => *start + 1,
            Bound::Unbounded => 0,
        };
        let lines = self.lines(range_bounds).map(|s| s.to_string()).collect();
        Snippet { start, lines }
    }
}

impl ExtractSnippet for Lines {}

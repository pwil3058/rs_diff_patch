// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::crange::CRange;
use crate::lines::{BasicLines, LazyLines};
use serde::{Deserialize, Serialize};
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
}

pub trait ExtractSnippet: BasicLines {
    fn extract_snippet(&self, range_bounds: impl RangeBounds<usize>) -> Snippet {
        let range = CRange::from(range_bounds);
        let start = range.start();
        let lines = self.lines(range).map(|s| s.to_string()).collect();
        Snippet { start, lines }
    }
}

impl ExtractSnippet for LazyLines {}

// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::crange::{CRange, Len};
use crate::lines::{DiffInputFile, LazyLines, LineIndices};

use std::collections::HashMap;
use std::ops::RangeBounds;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Serialize, Deserialize)]
pub struct Match(pub usize, pub usize, pub usize);

impl Len for Match {
    fn len(&self) -> usize {
        self.2
    }
}

impl Match {
    pub fn range_1(&self) -> CRange {
        CRange(self.0, self.0 + self.2)
    }

    pub fn range_2(&self) -> CRange {
        CRange(self.1, self.1 + self.2)
    }

    pub fn start_1(&self) -> usize {
        self.0
    }

    pub fn start_2(&self) -> usize {
        self.1
    }

    pub fn end_1(&self) -> usize {
        self.0 + self.2
    }

    pub fn end_2(&self) -> usize {
        self.1 + self.2
    }

    pub fn decr_starts(&mut self, arg: usize) {
        self.0 -= arg;
        self.1 -= arg;
        self.2 += arg;
    }

    pub fn incr_starts(&mut self, arg: usize) {
        self.0 += arg;
        self.1 += arg;
        self.2 -= arg;
    }

    pub fn incr_size(&mut self, arg: usize) {
        self.2 += arg;
    }

    pub fn decr_size(&mut self, arg: usize) {
        self.2 -= arg;
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OpCode {
    Equal(Match),
    Delete(CRange),
    Insert(CRange),
    Replace(CRange, CRange),
}

impl OpCode {
    pub fn starts_trimmed(&self, context: usize) -> Self {
        match self {
            OpCode::Equal(match_) => {
                if match_.len() > context {
                    OpCode::Equal(Match(
                        match_.end_1() - context,
                        match_.end_2() - context,
                        context,
                    ))
                } else {
                    *self
                }
            }
            _ => panic!("must be Equal to be trimmed"),
        }
    }

    pub fn ends_trimmed(&self, context: usize) -> Self {
        match self {
            OpCode::Equal(match_) => {
                if match_.len() > context {
                    OpCode::Equal(Match(match_.start_1(), match_.start_2(), context))
                } else {
                    *self
                }
            }
            _ => panic!("must be Equal to be trimmed"),
        }
    }

    pub fn split(&self, context: usize) -> Option<(Self, Self)> {
        match self {
            OpCode::Equal(match_) => {
                if match_.len() > context * 2 {
                    Some((self.ends_trimmed(context), self.starts_trimmed(context)))
                } else {
                    None
                }
            }
            _ => panic!("must be Equal to be split"),
        }
    }
}

#[derive(Debug, Default)]
pub struct Matcher {
    lines_1: LazyLines,
    lines_2: LazyLines,
    op_codes: Vec<OpCode>,
    lines_2_indices: LineIndices,
}

impl Matcher {
    pub fn new(lines_1: LazyLines, lines_2: LazyLines) -> Self {
        let mut matcher = Matcher::default();
        matcher.lines_1 = lines_1;
        matcher.lines_2 = lines_2;
        matcher.lines_2_indices = matcher.lines_2.get_line_indices();
        matcher.op_codes = matcher.generate_op_codes();
        matcher
    }

    /// Return an iterator over the OpCodes describing changes
    ///
    /// Example:
    /// ```
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::matcher::{Match, Matcher, OpCode};
    /// use OpCode::*;
    ///
    /// let lines_1 = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let lines_2 = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let matcher = Matcher::new(lines_1, lines_2);
    /// assert_eq!(
    ///     vec![Equal(Match(0,0,1)), Delete(CRange(1, 2)), Equal(Match(2, 1, 2)), Replace(CRange(4, 6), CRange(3, 5)), Equal(Match(6, 5, 5)), Insert(CRange(10, 11)), Equal(Match(11, 11, 2))],
    ///     matcher.op_codes().cloned().collect::<Vec<OpCode>>()
    /// );
    /// ```
    pub fn op_codes(&self) -> impl Iterator<Item = &OpCode> {
        self.op_codes.iter()
    }

    fn longest_match(
        &self,
        range_bounds_1: impl RangeBounds<usize>,
        range_bounds_2: impl RangeBounds<usize>,
    ) -> Option<Match> {
        let range_1 = self.lines_1.c_range(range_bounds_1);
        let range_2 = self.lines_2.c_range(range_bounds_2);

        let mut best_match = Match::default();

        let mut j_to_len = HashMap::<usize, usize>::new();
        for (i, line) in self.lines_1.lines(range_1).enumerate() {
            let index = i + range_1.start();
            let mut new_j_to_len = HashMap::<usize, usize>::new();
            if let Some(indices) = self.lines_2_indices.get(line) {
                for j in indices {
                    if j < &range_2.start() {
                        continue;
                    }
                    if j >= &range_2.end() {
                        break;
                    }

                    if j == &0 {
                        new_j_to_len.insert(0, 1);
                        if best_match.is_empty() {
                            best_match = Match(index, 0, 1);
                        }
                    } else {
                        let k = match j_to_len.get(&(j - 1)) {
                            Some(k) => *k + 1,
                            None => 1,
                        };
                        new_j_to_len.insert(*j, k);
                        if k > best_match.len() {
                            best_match = Match(index + 1 - k, j + 1 - k, k);
                        }
                    }
                }
            }
            j_to_len = new_j_to_len;
        }

        if best_match.is_empty() {
            None
        } else {
            let count = self
                .lines_1
                .lines_reversed(range_1.start()..best_match.start_1())
                .zip(
                    self.lines_2
                        .lines_reversed(range_2.start()..best_match.start_2()),
                )
                .take_while(|(a, b)| a == b)
                .count();
            best_match.decr_starts(count);

            let count = self
                .lines_1
                .lines(best_match.end_1() + 1..range_1.end())
                .zip(self.lines_2.lines(best_match.end_2() + 1..range_2.end()))
                .take_while(|(a, b)| a == b)
                .count();
            best_match.incr_size(count);

            Some(best_match)
        }
    }

    fn matching_blocks(&self) -> Vec<Match> {
        let mut lifo = vec![(CRange(0, self.lines_1.len()), CRange(0, self.lines_2.len()))];
        let mut raw_matching_blocks = vec![];
        while let Some((range_1, range_2)) = lifo.pop() {
            if let Some(match_) = self.longest_match(range_1.clone(), range_2.clone()) {
                if range_1.start() < match_.start_1() && range_2.start() < match_.start_2() {
                    lifo.push((
                        CRange(range_1.start(), match_.start_1()),
                        CRange(range_2.start(), match_.start_2()),
                    ))
                };
                if match_.end_1() < range_1.end() && match_.end_2() < range_2.end() {
                    lifo.push((
                        CRange(match_.end_1(), range_1.end()),
                        CRange(match_.end_2(), range_2.end()),
                    ))
                }
                raw_matching_blocks.push(match_);
            }
        }
        raw_matching_blocks.par_sort();

        let mut matching_blocks = vec![];
        let mut i = 0usize;
        while let Some(match_) = raw_matching_blocks.get(i) {
            let mut new_block = *match_;
            i += 1;
            while let Some(match_) = raw_matching_blocks.get(i) {
                if new_block.end_1() == match_.start_1() && new_block.end_2() == match_.start_2() {
                    new_block.incr_size(match_.len());
                    i += 1
                } else {
                    break;
                }
            }
            matching_blocks.push(new_block);
        }

        matching_blocks
    }

    pub fn generate_op_codes(&self) -> Vec<OpCode> {
        let mut op_codes = vec![];
        let mut i = 0usize;
        let mut j = 0usize;
        for match_ in self.matching_blocks().iter() {
            if i < match_.start_1() && j < match_.start_2() {
                op_codes.push(OpCode::Replace(
                    CRange(i, match_.start_1()),
                    CRange(j, match_.start_2()),
                ));
            } else if i < match_.start_1() {
                op_codes.push(OpCode::Delete(CRange(i, match_.start_1())));
            } else if j < match_.start_2() {
                op_codes.push(OpCode::Insert(CRange(j, match_.start_2())));
            }
            op_codes.push(OpCode::Equal(*match_));
            i = match_.end_1();
            j = match_.end_2();
        }
        if i < self.lines_1.len() && j < self.lines_2.len() {
            op_codes.push(OpCode::Replace(
                CRange(i, self.lines_1.len()),
                CRange(j, self.lines_2.len()),
            ));
        } else if i < self.lines_1.len() {
            op_codes.push(OpCode::Delete(CRange(i, self.lines_1.len())));
        } else if j < self.lines_2.len() {
            op_codes.push(OpCode::Insert(CRange(j, self.lines_2.len())));
        }

        op_codes
    }
}

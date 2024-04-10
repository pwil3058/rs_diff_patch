// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::crange::{CRange, Len};
use crate::lines::{DiffInputFile, LazyLines, LineIndices};

use std::collections::HashMap;
use std::iter::Enumerate;
use std::ops::{Deref, DerefMut, RangeBounds};
use std::slice::Iter;

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

    pub fn starts_trimmed(&self, arg: usize) -> Self {
        if self.2 > arg {
            Self(self.0 + self.2 - arg, self.1 + self.2 - arg, arg)
        } else {
            *self
        }
    }

    pub fn ends_trimmed(&self, arg: usize) -> Self {
        if self.2 > arg {
            Self(self.0, self.1, arg)
        } else {
            *self
        }
    }

    pub fn split(&self, arg: usize) -> Option<(Self, Self)> {
        if self.2 > arg * 2 {
            Some((self.ends_trimmed(arg), self.starts_trimmed(arg)))
        } else {
            None
        }
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
    Delete(CRange, usize),
    Insert(usize, CRange),
    Replace(CRange, CRange),
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
    ///     vec![Equal(Match(0,0,1)), Delete(CRange(1, 2), 1), Equal(Match(2, 1, 2)), Replace(CRange(4, 6), CRange(3, 5)), Equal(Match(6, 5, 5)), Insert(11, CRange(10, 11)), Equal(Match(11, 11, 2))],
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

    fn generate_op_codes(&self) -> Vec<OpCode> {
        let mut op_codes = vec![];
        let mut i = 0usize;
        let mut j = 0usize;
        for match_ in self.matching_blocks() {
            if i < match_.start_1() && j < match_.start_2() {
                op_codes.push(OpCode::Replace(
                    CRange(i, match_.start_1()),
                    CRange(j, match_.start_2()),
                ));
            } else if i < match_.start_1() {
                op_codes.push(OpCode::Delete(CRange(i, match_.start_1()), j));
            } else if j < match_.start_2() {
                op_codes.push(OpCode::Insert(i, CRange(j, match_.start_2())));
            }
            op_codes.push(OpCode::Equal(match_));
            i = match_.end_1();
            j = match_.end_2();
        }
        if i < self.lines_1.len() && j < self.lines_2.len() {
            op_codes.push(OpCode::Replace(
                CRange(i, self.lines_1.len()),
                CRange(j, self.lines_2.len()),
            ));
        } else if i < self.lines_1.len() {
            op_codes.push(OpCode::Delete(CRange(i, self.lines_1.len()), j));
        } else if j < self.lines_2.len() {
            op_codes.push(OpCode::Insert(i, CRange(j, self.lines_2.len())));
        }

        op_codes
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet(pub usize, pub Vec<String>);

pub trait ExtractSnippet<'a>: DiffInputFile<'a> {
    fn extract_snippet(&'a self, range_bounds: impl RangeBounds<usize>) -> Snippet {
        let range = self.c_range(range_bounds);
        let start = range.start();
        let lines = self.lines(range).map(|s| s.to_string()).collect();
        Snippet(start, lines)
    }
}

impl<'a> ExtractSnippet<'a> for LazyLines {}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IOpCode {
    Context(Snippet),
    Delete(Snippet),
    Insert(usize, Snippet),
    Replace(Snippet, Snippet),
}

impl Matcher {
    /// Return an iterator over the Independent OpCodes describing changes
    ///
    /// Example:
    /// ```
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::matcher::{Match, Matcher, IOpCode, Snippet};
    /// use IOpCode::*;
    ///
    /// let lines_1 = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let lines_2 = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let matcher = Matcher::new(lines_1, lines_2);
    /// let independent_op_codes = matcher.independent_op_codes(2);
    /// eprintln!("IOC: {independent_op_codes:?}");
    /// let expected = vec![
    ///     Context(Snippet(0, vec!["A\n".to_string()])),
    ///     Delete(Snippet(1, vec!["B\n".to_string()])),
    ///     Context(Snippet(2, vec!["C\n".to_string(), "D\n".to_string()])),
    ///     Replace(Snippet(4, vec!["E\n".to_string(), "F\n".to_string()]), Snippet(3, vec!["Ef\n".to_string(), "Fg\n".to_string()])),
    ///     Context(Snippet(6, vec!["G\n".to_string(), "H\n".to_string()])),
    ///     Context(Snippet(9, vec!["J\n".to_string(), "K\n".to_string()])),
    ///     Insert(11, Snippet(10, vec!["H\n".to_string()])),
    ///     Context(Snippet(11, vec!["L\n".to_string(), "M\n".to_string()]))
    /// ];
    /// assert_eq!(independent_op_codes.len(), expected.len());
    /// for (expected, got) in expected.iter().zip(independent_op_codes.iter()) {
    ///     assert_eq!(expected, got);
    /// }
    /// ```
    pub fn independent_op_codes(&self, context: usize) -> Vec<IOpCode> {
        let mut list = Vec::new();
        let last = self.op_codes.len() - 1;

        for (i, op_code) in self.op_codes.iter().enumerate() {
            use OpCode::*;
            match op_code {
                Equal(match_) => {
                    if i == 0 {
                        let range = match_.starts_trimmed(context).range_1();
                        list.push(IOpCode::Context(self.lines_1.extract_snippet(range)));
                    } else if i == last {
                        let range = match_.ends_trimmed(context).range_1();
                        list.push(IOpCode::Context(self.lines_1.extract_snippet(range)));
                    } else if let Some((head, tail)) = match_.split(context) {
                        list.push(IOpCode::Context(
                            self.lines_1.extract_snippet(head.range_1()),
                        ));
                        list.push(IOpCode::Context(
                            self.lines_1.extract_snippet(tail.range_1()),
                        ));
                    } else {
                        list.push(IOpCode::Context(
                            self.lines_1.extract_snippet(match_.range_1()),
                        ));
                    }
                }
                Delete(range, _) => {
                    list.push(IOpCode::Delete(self.lines_1.extract_snippet(*range)))
                }
                Insert(start, range) => {
                    let snippet = self.lines_2.extract_snippet(*range);
                    list.push(IOpCode::Insert(*start, snippet));
                }
                Replace(range_1, range_2) => {
                    let snippet_1 = self.lines_1.extract_snippet(*range_1);
                    let snippet_2 = self.lines_2.extract_snippet(*range_2);
                    list.push(IOpCode::Replace(snippet_1, snippet_2));
                }
            }
        }

        list
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct OpCodeChunk(pub Vec<OpCode>);

impl Deref for OpCodeChunk {
    type Target = Vec<OpCode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OpCodeChunk {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct OpCodeChunks<'a> {
    iter: Enumerate<Iter<'a, OpCode>>,
    tail: usize,
    context: usize,
    stash: Option<OpCode>,
}

impl<'a> Iterator for OpCodeChunks<'a> {
    type Item = OpCodeChunk;

    fn next(&mut self) -> Option<Self::Item> {
        use OpCode::Equal;
        let mut chunk = OpCodeChunk::default();
        if let Some(stashed) = self.stash {
            chunk.push(stashed);
            self.stash = None;
        }
        while let Some((i, op_code)) = self.iter.next() {
            match op_code {
                Equal(range) => {
                    if i == 0 || chunk.is_empty() {
                        // Trim starts
                        chunk.push(Equal(range.starts_trimmed(self.context)));
                    } else if i == self.tail {
                        // Trim size
                        chunk.push(Equal(range.ends_trimmed(self.context)));
                        return Some(chunk);
                    } else if let Some((head, tail)) = range.split(self.context) {
                        self.stash = Some(Equal(tail));
                        chunk.push(Equal(head));
                        return Some(chunk);
                    } else {
                        chunk.push(*op_code)
                    }
                }
                _ => {
                    chunk.push(*op_code);
                    if i == self.tail {
                        return Some(chunk);
                    }
                }
            }
        }
        None
    }
}

impl Matcher {
    /// Return an iterator over OpCodeChunks generated with the given `context` size.
    ///
    /// Example:
    /// ```
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::matcher::{Match, Matcher, OpCode, Snippet, OpCodeChunk};
    /// use OpCode::*;
    ///
    /// let lines_1 = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let lines_2 = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let matcher = Matcher::new(lines_1, lines_2);
    /// let expected = vec![
    ///     OpCodeChunk(vec![Equal(Match(0, 0, 1)), Delete(CRange(1, 2), 1), Equal(Match(2, 1, 2)), Replace(CRange(4, 6), CRange(3, 5)), Equal(Match(6, 5, 2))]),
    ///     OpCodeChunk(vec![Equal(Match(9, 8, 2)), Insert(11, CRange(10, 11)), Equal(Match(11, 11, 2))]),
    /// ];
    /// for (expected, got) in expected.iter().zip(matcher.op_code_chunks(2)) {
    ///     assert_eq!(*expected, got);
    /// }
    /// ```
    pub fn op_code_chunks(&self, context: usize) -> OpCodeChunks {
        OpCodeChunks {
            iter: self.op_codes.iter().enumerate(),
            tail: self.op_codes.len() - 1,
            stash: None,
            context,
        }
    }
}

struct UnifiedDiffChunks<'a> {
    iter: OpCodeChunks<'a>,
}

impl<'a> Iterator for UnifiedDiffChunks<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.iter.next()?;
        let mut udc = String::new();
        None
    }
}

impl Matcher {
    pub fn unified_diff_chunks(&self, context: usize) -> UnifiedDiffChunks {
        UnifiedDiffChunks {
            iter: self.op_code_chunks(context),
        }
    }
}

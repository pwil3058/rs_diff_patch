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
    pub fn before_range(&self) -> CRange {
        CRange(self.0, self.0 + self.2)
    }

    pub fn after_range(&self) -> CRange {
        CRange(self.1, self.1 + self.2)
    }

    pub fn before_start(&self) -> usize {
        self.0
    }

    pub fn after_start(&self) -> usize {
        self.1
    }

    pub fn before_end(&self) -> usize {
        self.0 + self.2
    }

    pub fn after_end(&self) -> usize {
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
fn longest_match<'a, L: DiffInputFile>(
    before: &L,
    after: &L,
    before_range_bounds: impl RangeBounds<usize>,
    after_range_bounds: impl RangeBounds<usize>,
    after_lines_indices: &LineIndices,
) -> Option<Match> {
    let before_range = before.c_range(before_range_bounds);
    let after_range = after.c_range(after_range_bounds);

    let mut best_match = Match::default();

    let mut j_to_len = HashMap::<usize, usize>::new();
    for (i, line) in before.lines(before_range).enumerate() {
        let index = i + before_range.start();
        let mut new_j_to_len = HashMap::<usize, usize>::new();
        if let Some(indices) = after_lines_indices.get(line) {
            for j in indices {
                if j < &after_range.start() {
                    continue;
                }
                if j >= &after_range.end() {
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
        let count = before
            .lines_reversed(before_range.start()..best_match.before_start())
            .zip(after.lines_reversed(after_range.start()..best_match.after_start()))
            .take_while(|(a, b)| a == b)
            .count();
        best_match.decr_starts(count);

        let count = before
            .lines(best_match.before_end() + 1..before_range.end())
            .zip(after.lines(best_match.after_end() + 1..after_range.end()))
            .take_while(|(a, b)| a == b)
            .count();
        best_match.incr_size(count);

        Some(best_match)
    }
}
fn matching_blocks<'a, L: DiffInputFile>(before: &L, after: &L) -> Vec<Match> {
    let after_line_indices = after.get_line_indices();
    let mut lifo = vec![(CRange(0, before.len()), CRange(0, after.len()))];
    let mut raw_matching_blocks = vec![];
    while let Some((before_range, after_range)) = lifo.pop() {
        if let Some(match_) = longest_match(
            before,
            after,
            before_range.clone(),
            after_range.clone(),
            &after_line_indices,
        ) {
            if before_range.start() < match_.before_start()
                && after_range.start() < match_.after_start()
            {
                lifo.push((
                    CRange(before_range.start(), match_.before_start()),
                    CRange(after_range.start(), match_.after_start()),
                ))
            };
            if match_.before_end() < before_range.end() && match_.after_end() < after_range.end() {
                lifo.push((
                    CRange(match_.before_end(), before_range.end()),
                    CRange(match_.after_end(), after_range.end()),
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
            if new_block.before_end() == match_.before_start()
                && new_block.after_end() == match_.after_start()
            {
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OpCode {
    Equal(Match),
    Delete(CRange, usize),
    Insert(usize, CRange),
    Replace(CRange, CRange),
}

fn generate_op_codes<'a, L: DiffInputFile>(before: &L, after: &L) -> Vec<OpCode> {
    let mut op_codes = vec![];
    let mut i = 0usize;
    let mut j = 0usize;
    for match_ in matching_blocks(before, after) {
        if i < match_.before_start() && j < match_.after_start() {
            op_codes.push(OpCode::Replace(
                CRange(i, match_.before_start()),
                CRange(j, match_.after_start()),
            ));
        } else if i < match_.before_start() {
            op_codes.push(OpCode::Delete(CRange(i, match_.before_start()), j));
        } else if j < match_.after_start() {
            op_codes.push(OpCode::Insert(i, CRange(j, match_.after_start())));
        }
        op_codes.push(OpCode::Equal(match_));
        i = match_.before_end();
        j = match_.after_end();
    }
    if i < before.len() && j < after.len() {
        op_codes.push(OpCode::Replace(
            CRange(i, before.len()),
            CRange(j, after.len()),
        ));
    } else if i < before.len() {
        op_codes.push(OpCode::Delete(CRange(i, before.len()), j));
    } else if j < after.len() {
        op_codes.push(OpCode::Insert(i, CRange(j, after.len())));
    }

    op_codes
}

#[derive(Debug, Default)]
pub struct Matcher<L: DiffInputFile> {
    before: L,
    after: L,
    op_codes: Vec<OpCode>,
}

impl<L: DiffInputFile> Matcher<L> {
    pub fn new(before: L, after: L) -> Self {
        let op_codes = { generate_op_codes(&before, &after) };
        Self {
            before,
            after,
            op_codes,
        }
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
    /// let before_lines = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let after_lines = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let matcher = Matcher::new(before_lines, after_lines);
    /// assert_eq!(
    ///     vec![Equal(Match(0,0,1)), Delete(CRange(1, 2), 1), Equal(Match(2, 1, 2)), Replace(CRange(4, 6), CRange(3, 5)), Equal(Match(6, 5, 5)), Insert(11, CRange(10, 11)), Equal(Match(11, 11, 2))],
    ///     matcher.op_codes().cloned().collect::<Vec<OpCode>>()
    /// );
    /// ```
    pub fn op_codes(&self) -> impl Iterator<Item = &OpCode> {
        self.op_codes.iter()
    }
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet(pub usize, pub Vec<String>);

pub trait ExtractSnippet: DiffInputFile {
    fn extract_snippet(&self, range_bounds: impl RangeBounds<usize>) -> Snippet {
        let range = self.c_range(range_bounds);
        let start = range.start();
        let lines = self.lines(range).map(|s| s.to_string()).collect();
        Snippet(start, lines)
    }
}

impl ExtractSnippet for LazyLines {}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IOpCode {
    Context(Snippet),
    Delete(Snippet),
    Insert(usize, Snippet),
    Replace(Snippet, Snippet),
}

impl<L: DiffInputFile + ExtractSnippet> Matcher<L> {
    /// Return an iterator over the Independent OpCodes describing changes
    ///
    /// Example:
    /// ```
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::matcher::{Match, Matcher, IOpCode, Snippet};
    /// use IOpCode::*;
    ///
    /// let before_lines = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let after_lines = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let matcher = Matcher::new(before_lines, after_lines);
    /// let independent_op_codes = matcher.independent_op_codes(2);
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
                        let range = match_.starts_trimmed(context).before_range();
                        list.push(IOpCode::Context(self.before.extract_snippet(range)));
                    } else if i == last {
                        let range = match_.ends_trimmed(context).before_range();
                        list.push(IOpCode::Context(self.before.extract_snippet(range)));
                    } else if let Some((head, tail)) = match_.split(context) {
                        list.push(IOpCode::Context(
                            self.before.extract_snippet(head.before_range()),
                        ));
                        list.push(IOpCode::Context(
                            self.before.extract_snippet(tail.before_range()),
                        ));
                    } else {
                        list.push(IOpCode::Context(
                            self.before.extract_snippet(match_.before_range()),
                        ));
                    }
                }
                Delete(range, _) => list.push(IOpCode::Delete(self.before.extract_snippet(*range))),
                Insert(start, range) => {
                    let snippet = self.after.extract_snippet(*range);
                    list.push(IOpCode::Insert(*start, snippet));
                }
                Replace(before_range, after_range) => {
                    let before_snippet = self.before.extract_snippet(*before_range);
                    let after_snippet = self.after.extract_snippet(*after_range);
                    list.push(IOpCode::Replace(before_snippet, after_snippet));
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

impl OpCodeChunk {
    fn starts(&self) -> (usize, usize) {
        if let Some(op_code) = self.0.first() {
            match op_code {
                OpCode::Delete(range, after_start) => (range.start(), *after_start),
                OpCode::Equal(match_) => (match_.before_start(), match_.after_start()),
                OpCode::Insert(before_start, after_range) => (*before_start, after_range.start()),
                OpCode::Replace(before_range, after_range) => {
                    (before_range.start(), after_range.start())
                }
            }
        } else {
            (0, 0)
        }
    }

    fn lengths(&self) -> (usize, usize) {
        let mut before_len = 0usize;
        let mut after_len = 0usize;
        for op_code in self.0.iter() {
            match op_code {
                OpCode::Delete(range, _) => {
                    before_len += range.len();
                }
                OpCode::Equal(match_) => {
                    before_len += match_.len();
                    after_len += match_.len();
                }
                OpCode::Insert(_, range) => {
                    after_len += range.len();
                }
                OpCode::Replace(before_range, after_range) => {
                    before_len += before_range.len();
                    after_len += after_range.len();
                }
            }
        }
        (before_len, after_len)
    }

    fn context_lengths(&self) -> (usize, usize) {
        let start = if let Some(op_code) = self.first() {
            match op_code {
                OpCode::Equal(match_) => match_.len(),
                _ => 0,
            }
        } else {
            0
        };
        let end = if let Some(op_code) = self.last() {
            match op_code {
                OpCode::Equal(match_) => match_.len(),
                _ => 0,
            }
        } else {
            0
        };
        (start, end)
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

impl<L: DiffInputFile> Matcher<L> {
    /// Return an iterator over OpCodeChunks generated with the given `context` size.
    ///
    /// Example:
    /// ```
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::matcher::{Match, Matcher, OpCode, Snippet, OpCodeChunk};
    /// use OpCode::*;
    ///
    /// let before_lines = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let after_lines = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let matcher = Matcher::new(before_lines, after_lines);
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

pub struct UnifiedDiffChunks<'a, L: DiffInputFile> {
    iter: OpCodeChunks<'a>,
    before: &'a L,
    after: &'a L,
}

impl<'a, L: DiffInputFile> Iterator for UnifiedDiffChunks<'a, L> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.iter.next()?;
        let (before_start, after_start) = chunk.starts();
        let (before_len, after_len) = chunk.lengths();
        let mut udc = format!(
            "@@ -{},{} +{},{} @@\n",
            before_start, before_len, after_start, after_len
        );
        for op_code in chunk.iter() {
            match op_code {
                OpCode::Delete(range, _) => {
                    for line in self.before.lines(*range) {
                        udc.push_str(&format!("-{line}"));
                    }
                }
                OpCode::Equal(match_) => {
                    for line in self.before.lines(match_.before_range()) {
                        udc.push_str(&format!(" {line}"));
                    }
                }
                OpCode::Insert(_, range) => {
                    for line in self.after.lines(*range) {
                        udc.push_str(&format!("+{line}"));
                    }
                }
                OpCode::Replace(range_before, range_after) => {
                    for line in self.before.lines(*range_before) {
                        udc.push_str(&format!("-{line}"));
                    }
                    for line in self.after.lines(*range_after) {
                        udc.push_str(&format!("+{line}"));
                    }
                }
            }
        }
        Some(udc)
    }
}

impl<L: DiffInputFile> Matcher<L> {
    /// Return an iterator over OpCodeChunks generated with the given `context` size.
    ///
    /// Example:
    /// ```
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::matcher::{Match, Matcher, OpCode, Snippet, OpCodeChunk};
    /// use OpCode::*;
    ///
    /// let before_lines = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let after_lines = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let matcher = Matcher::new(before_lines, after_lines);
    /// let expected = vec![
    ///     "@@ -0,8 +0,7 @@\n A\n-B\n C\n D\n-E\n-F\n+Ef\n+Fg\n G\n H\n",
    ///     "@@ -9,4 +8,5 @@\n J\n K\n+H\n L\n M\n"
    /// ];
    /// for (expected, got) in expected.iter().zip(matcher.unified_diff_chunks(2)) {
    ///     assert_eq!(*expected, got);
    /// }
    /// ```
    pub fn unified_diff_chunks<'a>(&'a self, context: usize) -> UnifiedDiffChunks<'a, L> {
        UnifiedDiffChunks {
            iter: self.op_code_chunks(context),
            before: &self.before,
            after: &self.after,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Chunk {
    pub context_lengths: (usize, usize),
    pub before: Snippet,
    pub after: Snippet,
}
pub struct Chunks<'a, L: DiffInputFile> {
    iter: OpCodeChunks<'a>,
    before: &'a L,
    after: &'a L,
}

impl<'a, L: DiffInputFile> Iterator for Chunks<'a, L> {
    type Item = Chunk;

    fn next(&mut self) -> Option<Self::Item> {
        let oc_chunk = self.iter.next()?;
        let (before_start, after_start) = oc_chunk.starts();
        let context_lengths = oc_chunk.context_lengths();
        let mut before_lines: Vec<String> = Vec::new();
        let mut after_lines: Vec<String> = Vec::new();
        for op_code in oc_chunk.iter() {
            match op_code {
                OpCode::Delete(range, _) => {
                    for line in self.before.lines(*range) {
                        before_lines.push(line.to_string())
                    }
                }
                OpCode::Equal(match_) => {
                    for line in self.before.lines(match_.before_range()) {
                        before_lines.push(line.to_string());
                        after_lines.push(line.to_string());
                    }
                }
                OpCode::Insert(_, range) => {
                    for line in self.after.lines(*range) {
                        after_lines.push(line.to_string())
                    }
                }
                OpCode::Replace(before_range, after_range) => {
                    for line in self.before.lines(*before_range) {
                        before_lines.push(line.to_string())
                    }
                    for line in self.after.lines(*after_range) {
                        after_lines.push(line.to_string())
                    }
                }
            }
        }

        let before = Snippet(before_start, before_lines);
        let after = Snippet(after_start, after_lines);

        Some(Chunk {
            context_lengths,
            before,
            after,
        })
    }
}

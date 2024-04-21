// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::crange::{CRange, Len};
use crate::lines::{BasicLines, DiffInputLines, LazyLines, LineIndices};
use crate::snippet::*;

use std::collections::HashMap;
use std::io;
use std::iter::Enumerate;
use std::ops::{Deref, DerefMut, RangeBounds};
use std::slice::Iter;

use crate::apply::ApplyChunkInto;
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

fn longest_match<'a, L: DiffInputLines>(
    before: &L,
    after: &L,
    before_range_bounds: impl RangeBounds<usize>,
    after_range_bounds: impl RangeBounds<usize>,
    after_lines_indices: &LineIndices,
) -> Option<Match> {
    let before_range = CRange::from(before_range_bounds);
    let after_range = CRange::from(after_range_bounds);

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

fn matching_blocks<'a, L: DiffInputLines>(before: &L, after: &L) -> Vec<Match> {
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

fn generate_op_codes<'a, L: DiffInputLines>(before: &L, after: &L) -> Vec<OpCode> {
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
pub struct Matcher<L: DiffInputLines> {
    before: L,
    after: L,
    op_codes: Vec<OpCode>,
}

impl<L: DiffInputLines> Matcher<L> {
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

    fn ends(&self) -> (usize, usize) {
        if let Some(op_code) = self.0.last() {
            match op_code {
                OpCode::Delete(range, after_start) => (range.end(), *after_start),
                OpCode::Equal(match_) => (match_.before_end(), match_.after_end()),
                OpCode::Insert(before_start, after_range) => (*before_start, after_range.end()),
                OpCode::Replace(before_range, after_range) => {
                    (before_range.end(), after_range.end())
                }
            }
        } else {
            (0, 0)
        }
    }

    fn ranges(&self) -> (CRange, CRange) {
        let (before_start, after_start) = self.starts();
        let (before_end, after_end) = self.ends();

        (
            CRange(before_start, before_end),
            CRange(after_start, after_end),
        )
    }

    fn lengths(&self) -> (usize, usize) {
        let (before_start, after_start) = self.starts();
        let (before_end, after_end) = self.ends();

        (before_end - before_start, after_end - after_start)
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
                        break;
                    } else if let Some((head, tail)) = range.split(self.context) {
                        self.stash = Some(Equal(tail));
                        chunk.push(Equal(head));
                        break;
                    } else {
                        chunk.push(*op_code)
                    }
                }
                _ => {
                    chunk.push(*op_code);
                }
            }
        }
        if chunk.is_empty() {
            None
        } else {
            Some(chunk)
        }
    }
}

impl<L: DiffInputLines> Matcher<L> {
    /// Return an iterator over OpCodeChunks generated with the given `context` size.
    ///
    /// Example:
    /// ```
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::matcher::{Match, Matcher, OpCode, OpCodeChunk};
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

#[derive(Debug, Default, PartialEq)]
pub struct UnifiedDiffChunk(pub Vec<String>);

impl Deref for UnifiedDiffChunk {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct UnifiedDiffChunks<'a, L: DiffInputLines> {
    iter: OpCodeChunks<'a>,
    before: &'a L,
    after: &'a L,
}

impl<'a, L: DiffInputLines> Iterator for UnifiedDiffChunks<'a, L> {
    type Item = UnifiedDiffChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.iter.next()?;
        let (before_start, after_start) = chunk.starts();
        let (before_len, after_len) = chunk.lengths();
        let mut udc = vec![format!(
            "@@ -{},{} +{},{} @@\n",
            before_start, before_len, after_start, after_len
        )];
        for op_code in chunk.iter() {
            match op_code {
                OpCode::Delete(range, _) => {
                    for line in self.before.lines(*range) {
                        udc.push(format!("-{line}"));
                    }
                }
                OpCode::Equal(match_) => {
                    for line in self.before.lines(match_.before_range()) {
                        udc.push(format!(" {line}"));
                    }
                }
                OpCode::Insert(_, range) => {
                    for line in self.after.lines(*range) {
                        udc.push(format!("+{line}"));
                    }
                }
                OpCode::Replace(range_before, range_after) => {
                    for line in self.before.lines(*range_before) {
                        udc.push(format!("-{line}"));
                    }
                    for line in self.after.lines(*range_after) {
                        udc.push(format!("+{line}"));
                    }
                }
            }
        }
        Some(UnifiedDiffChunk(udc))
    }
}

impl<L: DiffInputLines> Matcher<L> {
    /// Return an iterator over OpCodeChunks generated with the given `context` size.
    ///
    /// Example:
    /// ```
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::matcher::{Match, Matcher, OpCode, OpCodeChunk, UnifiedDiffChunk};
    /// use OpCode::*;
    ///
    /// let before_lines = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let after_lines = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let matcher = Matcher::new(before_lines, after_lines);
    /// let expected = vec![
    ///     UnifiedDiffChunk(vec![
    ///         "@@ -0,8 +0,7 @@\n".to_string(),
    ///         " A\n".to_string(),     
    ///         "-B\n".to_string(),
    ///         " C\n".to_string(),
    ///         " D\n".to_string(),
    ///         "-E\n".to_string(),
    ///         "-F\n".to_string(),
    ///         "+Ef\n".to_string(),
    ///         "+Fg\n".to_string(),
    ///         " G\n".to_string(),
    ///         " H\n".to_string()
    ///     ]),
    ///     UnifiedDiffChunk(vec![
    ///         "@@ -9,4 +8,5 @@\n".to_string(),
    ///         " J\n".to_string(),
    ///         " K\n".to_string(),
    ///         "+H\n".to_string(),
    ///         " L\n".to_string(),
    ///         " M\n".to_string(),
    ///     ])
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

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DiffChunk {
    pub context_lengths: (usize, usize),
    pub before: Snippet,
    pub after: Snippet,
}

pub trait MatchesAt: BasicLines {
    fn matches_at(&self, lines: &[String], at: usize) -> bool {
        if at < self.len() && self.len() - at >= lines.len() {
            lines.iter().zip(self.lines(at..)).all(|(b, a)| a == *b)
        } else {
            false
        }
    }
}

impl MatchesAt for LazyLines {}

use crate::apply::Applies;

impl DiffChunk {
    pub fn after(&self, reverse: bool) -> &Snippet {
        if reverse {
            &self.before
        } else {
            &self.after
        }
    }

    pub fn before(&self, reverse: bool) -> &Snippet {
        if reverse {
            &self.after
        } else {
            &self.before
        }
    }

    pub fn applies(&self, lines: &impl MatchesAt, offset: isize, reverse: bool) -> Option<Applies> {
        let before = self.before(reverse);
        let start = before.start as isize + offset;
        if !start.is_negative() && lines.matches_at(&before.lines, start as usize) {
            Some(Applies::Cleanly)
        } else {
            let max_reduction = self.context_lengths.0.max(self.context_lengths.1);
            for redn in 1..max_reduction {
                let start_redn = redn.min(self.context_lengths.0);
                let end_redn = redn.min(self.context_lengths.1);
                let adj_start = start + start_redn as isize;
                if !adj_start.is_negative()
                    && lines.matches_at(
                        &before.lines[start_redn..before.length(None) - end_redn],
                        adj_start as usize,
                    )
                {
                    return Some(Applies::WithReductions((start_redn, end_redn)));
                }
            }
            None
        }
    }

    pub fn applies_nearby(
        &self,
        lines: &impl MatchesAt,
        not_before: usize,
        next_chunk: Option<&DiffChunk>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, Applies)> {
        let before = self.before(reverse);
        let not_after = if let Some(next_chunk) = next_chunk {
            let next_chunk_before = if reverse {
                &next_chunk.after
            } else {
                &next_chunk.before
            };
            next_chunk_before
                .start
                .checked_add_signed(offset)
                .expect("overflow")
                - before.length(Some(self.context_lengths))
        } else {
            lines.len() - before.length(Some(self.context_lengths))
        };
        let mut backward_done = false;
        let mut forward_done = false;
        for i in 1isize.. {
            if !backward_done {
                let adjusted_offset = offset - i;
                if before.start as isize + adjusted_offset < not_before as isize {
                    backward_done = true;
                } else {
                    if let Some(applies) = self.applies(lines, adjusted_offset, reverse) {
                        return Some((-i, applies));
                    }
                }
            }
            if !forward_done {
                let adjusted_offset = offset + i;
                if before.start as isize + adjusted_offset < not_after as isize {
                    if let Some(applies) = self.applies(lines, adjusted_offset, reverse) {
                        return Some((i, applies));
                    }
                } else {
                    forward_done = true
                }
            }
            if forward_done && backward_done {
                break;
            }
        }
        None
    }
}

pub struct DiffChunks<'a, L: DiffInputLines> {
    iter: OpCodeChunks<'a>,
    before: &'a L,
    after: &'a L,
}

impl<'a, L: DiffInputLines> Iterator for DiffChunks<'a, L> {
    type Item = DiffChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let oc_chunk = self.iter.next()?;
        let (before_range, after_range) = oc_chunk.ranges();
        let context_lengths = oc_chunk.context_lengths();
        let before = Snippet {
            start: before_range.start(),
            lines: self
                .before
                .lines(before_range)
                .map(|l| l.to_string())
                .collect(),
        };
        let after = Snippet {
            start: after_range.start(),
            lines: self
                .after
                .lines(after_range)
                .map(|l| l.to_string())
                .collect(),
        };

        Some(DiffChunk {
            context_lengths,
            before,
            after,
        })
    }
}

impl<L: DiffInputLines> Matcher<L> {
    /// Return an iterator over OpCodeChunks generated with the given `context` size.
    ///
    /// Example:
    /// ```
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::matcher::{Match, Matcher, OpCode, OpCodeChunk, DiffChunk};
    /// use diff_lib::snippet::Snippet;
    /// use OpCode::*;
    ///
    /// let before_lines = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let after_lines = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let matcher = Matcher::new(before_lines, after_lines);
    /// let expected = vec![
    ///     DiffChunk {
    ///         context_lengths: (1, 2),
    ///         before: Snippet{start: 0, lines: vec!["A\n".to_string(), "B\n".to_string(), "C\n".to_string(), "D\n".to_string(), "E\n".to_string(), "F\n".to_string(), "G\n".to_string(), "H\n".to_string()]},
    ///         after: Snippet{start: 0, lines:vec!["A\n".to_string(), "C\n".to_string(), "D\n".to_string(), "Ef\n".to_string(), "Fg\n".to_string(), "G\n".to_string(), "H\n".to_string()]}
    ///     },
    ///     DiffChunk {
    ///         context_lengths: (2, 2),
    ///         before: Snippet{start: 9, lines: vec!["J\n".to_string(), "K\n".to_string(), "L\n".to_string(), "M\n".to_string()]},
    ///         after: Snippet{start: 8, lines: vec!["J\n".to_string(), "K\n".to_string(), "H\n".to_string(), "L\n".to_string(), "M\n".to_string()]}
    ///     },
    /// ];
    /// for (expected, got) in expected.iter().zip(matcher.diff_chunks(2)) {
    ///     assert_eq!(*expected, got);
    /// }
    /// ```
    pub fn diff_chunks<'a>(&'a self, context: usize) -> DiffChunks<'a, L> {
        DiffChunks {
            iter: self.op_code_chunks(context),
            before: &self.before,
            after: &self.after,
        }
    }
}
use crate::apply::ProgressData;

impl DiffChunk {
    pub fn apply_into<'a, L, W>(
        &self,
        pd: &mut ProgressData<'a, L>,
        into: &mut W,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> io::Result<()>
    where
        L: BasicLines,
        W: io::Write,
    {
        let end = self.before(reverse).start(pd.offset, reductions);
        for line in pd.lines.lines(pd.consumed..end) {
            into.write_all(line.as_bytes())?;
        }
        for line in self.after(reverse).lines(reductions) {
            into.write_all(line.as_bytes())?;
        }
        pd.consumed = end + self.before.length(reductions);
        Ok(())
    }

    pub fn already_applied_into<'a, L, W>(
        &self,
        pd: &mut ProgressData<'a, L>,
        into: &mut W,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> io::Result<()>
    where
        L: BasicLines,
        W: io::Write,
    {
        let after = self.after(reverse);
        let end = after.start(pd.offset, reductions) + after.length(reductions);
        for line in pd.lines.lines(pd.consumed..end) {
            into.write_all(line.as_bytes())?;
        }
        pd.consumed = end;
        Ok(())
    }
}

#[cfg(test)]
mod matcher_tests;

// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::collections::HashMap;
use std::iter::Peekable;
use std::ops::{Deref, DerefMut};
use std::slice::Iter;

use rayon::prelude::ParallelSliceMut;

use crate::common_subsequence::*;
use crate::range::*;
use crate::sequence::{ByteItemIndices, ContentItemIndices, Seq, StringItemIndices};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Change {
    NoChange(CommonSubsequence),
    Delete(Range, usize),
    Insert(usize, Range),
    Replace(Range, Range),
}

pub trait ChangeBasics {
    fn before_start(&self, reverse: bool) -> usize;
    fn before_end(&self, reverse: bool) -> usize;

    fn before_length(&self, reverse: bool) -> usize {
        self.before_end(reverse) - self.before_start(reverse)
    }

    fn before_range(&self, reductions: Option<(u8, u8)>, reverse: bool) -> Range {
        if let Some(reductions) = reductions {
            Range(
                self.before_start(reverse) + reductions.0 as usize,
                self.before_end(reverse) - reductions.1 as usize,
            )
        } else {
            Range(self.before_start(reverse), self.before_end(reverse))
        }
    }
    fn my_before_range(&self, reductions: Option<(u8, u8)>, reverse: bool) -> Range {
        let length = self.before_length(reverse);
        if let Some(reductions) = reductions {
            Range(reductions.0 as usize, length - reductions.1 as usize)
        } else {
            Range(0, length)
        }
    }

    fn after_start(&self, reverse: bool) -> usize {
        self.before_start(!reverse)
    }

    fn after_end(&self, reverse: bool) -> usize {
        self.before_end(!reverse)
    }

    fn after_length(&self, reverse: bool) -> usize {
        self.before_length(!reverse)
    }

    fn after_range(&self, reductions: Option<(u8, u8)>, reverse: bool) -> Range {
        self.before_range(reductions, !reverse)
    }

    fn my_after_range(&self, reductions: Option<(u8, u8)>, reverse: bool) -> Range {
        self.my_before_range(reductions, !reverse)
    }
}

impl ChangeBasics for Change {
    fn before_start(&self, reverse: bool) -> usize {
        if reverse {
            match self {
                Change::NoChange(common_subsequence) => common_subsequence.after_start(),
                Change::Delete(_, start) => *start,
                Change::Insert(_, after_range) => after_range.start(),
                Change::Replace(_, after_range) => after_range.start(),
            }
        } else {
            match self {
                Change::NoChange(common_subsequence) => common_subsequence.before_start(),
                Change::Delete(before_range, _) => before_range.start(),
                Change::Insert(start, _) => *start,
                Change::Replace(before_range, _) => before_range.start(),
            }
        }
    }

    fn before_end(&self, reverse: bool) -> usize {
        if reverse {
            match self {
                Change::NoChange(common_subsequence) => common_subsequence.after_end(),
                Change::Delete(_, end) => *end,
                Change::Insert(_, after_range) => after_range.end(),
                Change::Replace(_, after_range) => after_range.end(),
            }
        } else {
            match self {
                Change::NoChange(common_subsequence) => common_subsequence.before_end(),
                Change::Delete(before_range, _) => before_range.end(),
                Change::Insert(end, _) => *end,
                Change::Replace(before_range, _) => before_range.end(),
            }
        }
    }
}

#[derive(Debug)]
pub struct ChangesGenerator<'a, T: PartialEq + Clone, I: ContentItemIndices<T>> {
    before: &'a Seq<T>,
    after: &'a Seq<T>,
    before_content_indices: Box<I>,
}

impl<'a, T: PartialEq + Clone, I: ContentItemIndices<T>> ChangesGenerator<'a, T, I> {
    pub fn new(before: &'a Seq<T>, after: &'a Seq<T>) -> Self {
        let before_content_indices = ContentItemIndices::<T>::generate_from(before);
        Self {
            before,
            after,
            before_content_indices,
        }
    }
}

impl<'a, T: PartialEq + Clone, I: ContentItemIndices<T>> ChangesGenerator<'a, T, I> {
    /// Find the longest common subsequences in the given subsequences
    ///
    /// Example:
    /// ```
    /// use pw_diff_lib::sequence::{Seq, ContentItemIndices, StringItemIndices};
    /// use pw_diff_lib::changes::ChangesGenerator;
    /// use pw_diff_lib::range::Range;
    /// use pw_diff_lib::common_subsequence::CommonSubsequence;
    /// let before = Seq::<String>::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\n");
    /// let after = Seq::<String>::from("X\nY\nZ\nC\nD\nE\nH\nI\nX\n");
    /// let generator = ChangesGenerator::<String, StringItemIndices>::new(&before, &after);
    /// assert_eq!(Some(CommonSubsequence(2,3,3)), generator.longest_common_subsequence(before.range_from(0), after.range_from(0)));
    /// ```
    pub fn longest_common_subsequence(
        &self,
        before_range: Range,
        after_range: Range,
    ) -> Option<CommonSubsequence> {
        let mut best_lcs = CommonSubsequence::default();

        let mut j_to_len = HashMap::<isize, usize>::new();
        for (i, item) in self.after.subsequence(after_range).enumerate() {
            let index = i + after_range.start();
            let mut new_j_to_len = HashMap::<isize, usize>::new();
            if let Some(indices) = self.before_content_indices.indices(item) {
                for j in indices {
                    if j < &before_range.start() {
                        continue;
                    }
                    if j >= &before_range.end() {
                        break;
                    }

                    let k = match j_to_len.get(&(*j as isize - 1)) {
                        Some(k) => *k + 1,
                        None => 1,
                    };
                    new_j_to_len.insert(*j as isize, k);
                    if k > best_lcs.len() {
                        best_lcs = CommonSubsequence(j + 1 - k, index + 1 - k, k);
                    }
                }
            }
            j_to_len = new_j_to_len;
        }

        if best_lcs.is_empty() {
            None
        } else {
            let count = self
                .before
                .subsequence(Range(before_range.start(), best_lcs.before_start()))
                .rev()
                .zip(
                    self.after
                        .subsequence(Range(after_range.start(), best_lcs.after_start()))
                        .rev(),
                )
                .take_while(|(a, b)| a == b)
                .count();
            best_lcs.incr_size_moving_starts(
                count
                    .min(best_lcs.before_start())
                    .min(best_lcs.after_start()),
            );

            if best_lcs.before_end() + 1 < before_range.end()
                && best_lcs.after_end() + 1 < after_range.end()
            {
                let count = self
                    .before
                    .subsequence(Range(best_lcs.before_end() + 1, before_range.end()))
                    .zip(
                        self.after
                            .subsequence(Range(best_lcs.after_end() + 1, after_range.end())),
                    )
                    .take_while(|(a, b)| a == b)
                    .count();
                best_lcs.incr_size_moving_ends(count);
            }

            Some(best_lcs)
        }
    }

    fn longest_common_subsequences(&self) -> Vec<CommonSubsequence> {
        let mut lifo = vec![(self.before.range_from(0), self.after.range_from(0))];
        let mut raw_lcses = vec![];
        while let Some((before_range, after_range)) = lifo.pop() {
            if let Some(lcs) = self.longest_common_subsequence(before_range, after_range) {
                if before_range.start() < lcs.before_start()
                    && after_range.start() < lcs.after_start()
                {
                    lifo.push((
                        Range(before_range.start(), lcs.before_start()),
                        Range(after_range.start(), lcs.after_start()),
                    ))
                };
                if lcs.before_end() < before_range.end() && lcs.after_end() < after_range.end() {
                    lifo.push((
                        Range(lcs.before_end(), before_range.end()),
                        Range(lcs.after_end(), after_range.end()),
                    ))
                }
                raw_lcses.push(lcs);
            }
        }
        raw_lcses.par_sort();

        let mut lcses = vec![];
        let mut i = 0usize;
        while let Some(lcs) = raw_lcses.get(i) {
            let mut new_lcs = *lcs;
            i += 1;
            while let Some(lcs) = raw_lcses.get(i) {
                if new_lcs.before_end() == lcs.before_start()
                    && new_lcs.after_end() == lcs.after_start()
                {
                    new_lcs.incr_size_moving_ends(lcs.len());
                    i += 1
                } else {
                    break;
                }
            }
            lcses.push(new_lcs);
        }

        lcses
    }

    /// Return an iterator over the Mods describing changes
    ///
    /// Example:
    /// ```
    /// use pw_diff_lib::range::Range;
    /// use pw_diff_lib::sequence::{Seq, ContentItemIndices, StringItemIndices};
    /// use pw_diff_lib::common_subsequence::CommonSubsequence;
    /// use pw_diff_lib::changes::ChangesGenerator;
    /// use pw_diff_lib::changes::Change::*;
    ///
    /// let before_lines = Seq::<String>::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let after_lines = Seq::<String>::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let changes = ChangesGenerator::<String, StringItemIndices>::new(&before_lines, &after_lines).generate();
    /// assert_eq!(
    ///     vec![
    ///         NoChange(CommonSubsequence(0,0,1)), Delete(Range(1, 2), 1),
    ///         NoChange(CommonSubsequence(2, 1, 2)), Replace(Range(4, 6), Range(3, 5)),
    ///         NoChange(CommonSubsequence(6, 5, 5)), Insert(11, Range(10, 11)),
    ///         NoChange(CommonSubsequence(11, 11, 2))
    ///     ],
    ///     changes
    /// );
    /// ```
    pub fn generate(&self) -> Vec<Change> {
        let mut changes = vec![];
        let mut i = 0usize;
        let mut j = 0usize;

        for lcs in self.longest_common_subsequences() {
            if i < lcs.before_start() && j < lcs.after_start() {
                changes.push(Change::Replace(
                    Range(i, lcs.before_start()),
                    Range(j, lcs.after_start()),
                ));
            } else if i < lcs.before_start() {
                changes.push(Change::Delete(
                    Range(i, lcs.before_start()),
                    lcs.after_start(),
                ));
            } else if j < lcs.after_start() {
                changes.push(Change::Insert(
                    lcs.before_start(),
                    Range(j, lcs.after_start()),
                ));
            }
            changes.push(Change::NoChange(lcs));
            i = lcs.before_end();
            j = lcs.after_end();
        }
        if i < self.before.len() && j < self.after.len() {
            changes.push(Change::Replace(
                self.before.range_from(i),
                self.after.range_from(j),
            ));
        } else if i < self.before.len() {
            changes.push(Change::Delete(
                self.before.range_from(i),
                self.after.len(),
            ));
        } else if j < self.after.len() {
            changes.push(Change::Insert(
                self.before.len(),
                self.after.range_from(j),
            ));
        }

        changes
    }
}

#[derive(Debug, Default)]
pub struct Changes<T: PartialEq + Clone> {
    pub before: Seq<T>,
    pub after: Seq<T>,
    pub changes: Vec<Change>,
}

impl Changes<String> {
    pub fn new(before: Seq<String>, after: Seq<String>) -> Self {
        let changes =
            ChangesGenerator::<String, StringItemIndices>::new(&before, &after).generate();
        Self {
            before,
            after,
            changes,
        }
    }
}

impl Changes<u8> {
    pub fn new(before: Seq<u8>, after: Seq<u8>) -> Self {
        let changes = ChangesGenerator::<u8, ByteItemIndices>::new(&before, &after).generate();
        Self {
            before,
            after,
            changes,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ChangeClump<'a, T: PartialEq + Clone> {
    pub before: &'a Seq<T>,
    pub after: &'a Seq<T>,
    pub changes: Vec<Change>,
}

impl<'a, T: PartialEq + Clone> Deref for ChangeClump<'a, T> {
    type Target = Vec<Change>;

    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T: PartialEq + Clone> DerefMut for ChangeClump<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.changes
    }
}

impl<'a, T: PartialEq + Clone> ChangeBasics for ChangeClump<'a, T> {
    fn before_start(&self, reverse: bool) -> usize {
        if let Some(change) = self.changes.first() {
            change.before_start(reverse)
        } else {
            0
        }
    }

    fn before_end(&self, reverse: bool) -> usize {
        if let Some(change) = self.changes.first() {
            change.before_end(reverse)
        } else {
            0
        }
    }
}

impl<'a, T: PartialEq + Clone> ChangeClump<'a, T> {
    pub fn starts(&self) -> (usize, usize) {
        use Change::*;
        if let Some(change) = self.changes.first() {
            match change {
                Delete(range, after_start) => (range.start(), *after_start),
                NoChange(match_) => (match_.before_start(), match_.after_start()),
                Insert(before_start, after_range) => (*before_start, after_range.start()),
                Replace(before_range, after_range) => (before_range.start(), after_range.start()),
            }
        } else {
            (0, 0)
        }
    }

    pub fn ends(&self) -> (usize, usize) {
        use Change::*;
        if let Some(op_code) = self.changes.last() {
            match op_code {
                Delete(range, after_start) => (range.end(), *after_start),
                NoChange(match_) => (match_.before_end(), match_.after_end()),
                Insert(before_start, after_range) => (*before_start, after_range.end()),
                Replace(before_range, after_range) => (before_range.end(), after_range.end()),
            }
        } else {
            (0, 0)
        }
    }

    pub fn ranges(&self) -> (Range, Range) {
        let (before_start, after_start) = self.starts();
        let (before_end, after_end) = self.ends();

        (
            Range(before_start, before_end),
            Range(after_start, after_end),
        )
    }

    pub fn context_lengths(&self) -> (u8, u8) {
        use Change::NoChange;
        let start = if let Some(change) = self.first() {
            match change {
                NoChange(match_) => match_.len(),
                _ => 0,
            }
        } else {
            0
        };
        let end = if let Some(op_code) = self.last() {
            match op_code {
                NoChange(match_) => match_.len(),
                _ => 0,
            }
        } else {
            0
        };
        (start as u8, end as u8)
    }
}

pub struct ChangeClumpIter<'a, T: PartialEq + Clone> {
    pub before: &'a Seq<T>,
    pub after: &'a Seq<T>,
    iter: Peekable<Iter<'a, Change>>,
    context: u8,
    stash: Option<CommonSubsequence>,
}

impl<'a, T: PartialEq + Clone> Iterator for ChangeClumpIter<'a, T> {
    type Item = ChangeClump<'a, T>;

    fn next(&mut self) -> Option<Self::Item> {
        use Change::NoChange;
        let mut changes = vec![];
        if let Some(stashed) = self.stash {
            changes.push(NoChange(stashed));
            self.stash = None;
        }
        while let Some(change) = self.iter.next() {
            match change {
                NoChange(common_sequence) => {
                    if changes.is_empty() {
                        if self.iter.peek().is_some() {
                            changes.push(NoChange(common_sequence.starts_trimmed(self.context)));
                        }
                    } else if self.iter.peek().is_none() {
                        changes.push(NoChange(common_sequence.ends_trimmed(self.context)));
                        break;
                    } else if let Some((head, tail)) = common_sequence.split(self.context) {
                        self.stash = Some(tail);
                        changes.push(NoChange(head));
                        break;
                    } else {
                        changes.push(*change)
                    }
                }
                _ => {
                    changes.push(*change);
                }
            }
        }
        if changes.is_empty() {
            None
        } else {
            Some(ChangeClump {
                before: self.before,
                after: self.after,
                changes,
            })
        }
    }
}

impl<T: PartialEq + Clone> Changes<T> {
    /// Return an iterator over ModificationClumps generated with the given `context` size.
    ///
    /// Example:
    ///
    /// ```
    /// use pw_diff_lib::common_subsequence::CommonSubsequence;
    /// use pw_diff_lib::sequence::*;
    /// use pw_diff_lib::changes::{ChangeClump, Changes,Change};
    /// use pw_diff_lib::range::Range;
    /// use Change::*;
    ///
    /// let before = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    /// let after = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    /// let before_lines = Seq::<String>::from(before);
    /// let after_lines = Seq::<String>::from(after);
    /// let changes = Changes::<String>::new(before_lines, after_lines);
    /// let change_clumps: Vec<_> = changes.change_clumps(2).collect();
    /// assert_eq!(
    ///     change_clumps,
    ///     vec![
    ///         ChangeClump{
    ///             before: &Seq::<String>::from(before),
    ///             after: &Seq::<String>::from(after),
    ///             changes: vec![
    ///                 NoChange(CommonSubsequence(0, 0, 1)),
    ///                 Delete(Range(1, 2), 1),
    ///                 NoChange(CommonSubsequence(2, 1, 2)),
    ///                 Replace(Range(4, 6), Range(3, 5)),
    ///                 NoChange(CommonSubsequence(6, 5, 2))
    ///             ]
    ///         },
    ///         ChangeClump{
    ///             before: &Seq::<String>::from(before),
    ///             after: &Seq::<String>::from(after),
    ///             changes: vec![
    ///                 NoChange(CommonSubsequence(9, 8, 2)),
    ///                 Insert(11, Range(10, 11)),
    ///                 NoChange(CommonSubsequence(11, 11, 2))
    ///             ]
    ///         },
    ///     ]
    /// );
    /// ```
    pub fn change_clumps<'a>(&'a self, context: u8) -> ChangeClumpIter<'a, T> {
        ChangeClumpIter {
            before: &self.before,
            after: &self.after,
            iter: self.changes.iter().peekable(),
            context,
            stash: None,
        }
    }
}

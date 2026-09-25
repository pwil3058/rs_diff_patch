// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::iter::Peekable;
use std::ops::{Deref, DerefMut};
use std::slice::Iter;

use crate::{common_subsequence::*, range::*, sequence::*};

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
                Change::NoChange(common_subsequence) => common_subsequence.right_start(),
                Change::Delete(_, start) => *start,
                Change::Insert(_, after_range) => after_range.start(),
                Change::Replace(_, after_range) => after_range.start(),
            }
        } else {
            match self {
                Change::NoChange(common_subsequence) => common_subsequence.left_start(),
                Change::Delete(before_range, _) => before_range.start(),
                Change::Insert(start, _) => *start,
                Change::Replace(before_range, _) => before_range.start(),
            }
        }
    }

    fn before_end(&self, reverse: bool) -> usize {
        if reverse {
            match self {
                Change::NoChange(common_subsequence) => common_subsequence.right_end(),
                Change::Delete(_, end) => *end,
                Change::Insert(_, after_range) => after_range.end(),
                Change::Replace(_, after_range) => after_range.end(),
            }
        } else {
            match self {
                Change::NoChange(common_subsequence) => common_subsequence.left_end(),
                Change::Delete(before_range, _) => before_range.end(),
                Change::Insert(end, _) => *end,
                Change::Replace(before_range, _) => before_range.end(),
            }
        }
    }
}

#[derive(Debug)]
pub struct Changes<'a, T: PartialEq + Eq + Clone + std::hash::Hash> {
    pub before: &'a Seq<T>,
    pub after: &'a Seq<T>,
    pub changes: Box<[Change]>,
}

impl<'a, T: PartialEq + Eq + Clone + std::hash::Hash> Changes<'a, T> {
    pub fn new(before: &'a Seq<T>, after: &'a Seq<T>) -> Self {
        let mut changes = vec![];
        let mut i = 0usize;
        let mut j = 0usize;
        for lcs in crate::longest_common_subsequences::<T>(before, after).iter() {
            if i < lcs.left_start() && j < lcs.right_start() {
                changes.push(Change::Replace(
                    crate::range::Range(i, lcs.left_start()),
                    crate::range::Range(j, lcs.right_start()),
                ));
            } else if i < lcs.left_start() {
                changes.push(Change::Delete(
                    crate::range::Range(i, lcs.left_start()),
                    lcs.right_start(),
                ));
            } else if j < lcs.right_start() {
                changes.push(Change::Insert(
                    lcs.left_start(),
                    crate::range::Range(j, lcs.right_start()),
                ));
            }
            changes.push(Change::NoChange(*lcs));
            i = lcs.left_end();
            j = lcs.right_end();
        }
        if i < before.len() && j < after.len() {
            changes.push(Change::Replace(before.range_from(i), after.range_from(j)));
        } else if i < before.len() {
            changes.push(Change::Delete(before.range_from(i), after.len()));
        } else if j < after.len() {
            changes.push(Change::Insert(before.len(), after.range_from(j)));
        }
        Changes {
            before,
            after,
            changes: changes.into_boxed_slice(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ChangeClump<'a, T: PartialEq + Clone> {
    pub before: &'a Seq<T>,
    pub after: &'a Seq<T>,
    pub changes: Box<[Change]>,
}

impl<'a, T: PartialEq + Clone> Deref for ChangeClump<'a, T> {
    type Target = [Change];

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
                NoChange(match_) => (match_.left_start(), match_.right_start()),
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
                NoChange(match_) => (match_.left_end(), match_.right_end()),
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
        let start = if let Some(NoChange(m)) = self.first() {
            m.len()
        } else {
            0
        };
        let end = if let Some(NoChange(m)) = self.last() {
            m.len()
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
                changes: changes.into_boxed_slice(),
            })
        }
    }
}

impl<'a, T: PartialEq + Eq + Clone + std::hash::Hash> Changes<'a, T> {
    /// Return an iterator over ChangeClumps generated with the given `context` size.
    ///
    /// Example:
    ///
    /// ```
    /// use Change::*;
    /// use longest_common_subsequence::changes::{Change, ChangeClump, Changes};
    /// use longest_common_subsequence::common_subsequence::CommonSubsequence;
    /// use longest_common_subsequence::range::Range;
    /// use longest_common_subsequence::sequence::*;
    ///
    /// let before = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    /// let after = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    /// let before_lines = Seq::<String>::from_iter(before.split_inclusive('\n').map(|s| s.to_string()));
    /// let after_lines = Seq::<String>::from_iter(after.split_inclusive('\n').map(|s| s.to_string()));
    /// let changes = Changes::<String>::new(&before_lines, &after_lines);
    /// let change_clumps: Vec<_> = changes.change_clumps(2).collect();
    /// assert_eq!(
    ///     change_clumps,
    ///     vec![
    ///         ChangeClump{
    ///             before: &before_lines,
    ///             after: &after_lines,
    ///             changes: vec![
    ///                 NoChange(CommonSubsequence(0, 0, 1)),
    ///                 Delete(Range(1, 2), 1),
    ///                 NoChange(CommonSubsequence(2, 1, 2)),
    ///                 Replace(Range(4, 6), Range(3, 5)),
    ///                 NoChange(CommonSubsequence(6, 5, 2))
    ///             ].into()
    ///         },
    ///         ChangeClump{
    ///             before: &before_lines,
    ///             after: &after_lines,
    ///             changes: vec![
    ///                 NoChange(CommonSubsequence(9, 8, 2)),
    ///                 Insert(11, Range(10, 11)),
    ///                 NoChange(CommonSubsequence(11, 11, 2))
    ///             ].into()
    ///         },
    ///     ]
    /// );
    /// ```
    pub fn change_clumps(&'a self, context: u8) -> ChangeClumpIter<'a, T> {
        ChangeClumpIter {
            before: self.before,
            after: self.after,
            iter: self.changes.iter().peekable(),
            context,
            stash: None,
        }
    }
}

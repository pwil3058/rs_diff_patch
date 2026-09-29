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

    #[inline]
    fn before_length(&self, reverse: bool) -> usize {
        self.before_end(reverse)
            .saturating_sub(self.before_start(reverse))
    }

    fn before_range(&self, reductions: Option<(u8, u8)>, reverse: bool) -> Range {
        if let Some(reductions) = reductions {
            Range(
                self.before_start(reverse) + reductions.0 as usize,
                self.before_end(reverse)
                    .saturating_sub(reductions.1 as usize),
            )
        } else {
            Range(self.before_start(reverse), self.before_end(reverse))
        }
    }

    fn my_before_range(&self, reductions: Option<(u8, u8)>, reverse: bool) -> Range {
        let length = self.before_length(reverse);
        if let Some(reductions) = reductions {
            Range(
                reductions.0 as usize,
                length.saturating_sub(reductions.1 as usize),
            )
        } else {
            Range(0, length)
        }
    }

    #[inline]
    fn after_start(&self, reverse: bool) -> usize {
        self.before_start(!reverse)
    }

    #[inline]
    fn after_end(&self, reverse: bool) -> usize {
        self.before_end(!reverse)
    }

    #[inline]
    fn after_length(&self, reverse: bool) -> usize {
        self.before_length(!reverse)
    }

    #[inline]
    fn after_range(&self, reductions: Option<(u8, u8)>, reverse: bool) -> Range {
        self.before_range(reductions, !reverse)
    }

    #[inline]
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
pub struct Changes<'a, T: PartialEq + Eq + Clone + std::hash::Hash + Sync> {
    pub before: &'a Seq<T>,
    pub after: &'a Seq<T>,
    pub changes: Box<[Change]>,
}

impl<'a, T: PartialEq + Eq + Clone + std::hash::Hash + Sync> Changes<'a, T> {
    pub fn new(before: &'a Seq<T>, after: &'a Seq<T>) -> Self {
        let raw_lcs = crate::longest_common_subsequences::<T>(before, after);
        // Pre-allocate to prevent mid-loop resizing vectors
        let mut changes = Vec::with_capacity(raw_lcs.len() * 2 + 1);

        let mut i = 0usize;
        let mut j = 0usize;

        for lcs in raw_lcs.iter() {
            if i < lcs.left_start() && j < lcs.right_start() {
                changes.push(Change::Replace(
                    Range(i, lcs.left_start()),
                    Range(j, lcs.right_start()),
                ));
            } else if i < lcs.left_start() {
                changes.push(Change::Delete(
                    Range(i, lcs.left_start()),
                    lcs.right_start(),
                ));
            } else if j < lcs.right_start() {
                changes.push(Change::Insert(
                    lcs.left_start(),
                    Range(j, lcs.right_start()),
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
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.changes
    }
}

impl<'a, T: PartialEq + Clone> DerefMut for ChangeClump<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.changes
    }
}

impl<'a, T: PartialEq + Clone> ChangeBasics for ChangeClump<'a, T> {
    fn before_start(&self, reverse: bool) -> usize {
        self.changes.first().map_or(0, |c| c.before_start(reverse))
    }

    fn before_end(&self, reverse: bool) -> usize {
        self.changes.last().map_or(0, |c| c.before_end(reverse))
    }
}

impl<'a, T: PartialEq + Clone> ChangeClump<'a, T> {
    #[inline]
    pub fn starts(&self) -> (usize, usize) {
        (self.before_start(false), self.after_start(false))
    }

    #[inline]
    pub fn ends(&self) -> (usize, usize) {
        (self.before_end(false), self.after_end(false))
    }

    #[inline]
    pub fn ranges(&self) -> (Range, Range) {
        (
            self.before_range(None, false),
            self.after_range(None, false),
        )
    }

    pub fn context_lengths(&self) -> (u8, u8) {
        let start = match self.changes.first() {
            Some(Change::NoChange(m)) => m.len(),
            _ => 0,
        };
        let end = match self.changes.last() {
            Some(Change::NoChange(m)) => m.len(),
            _ => 0,
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
        let mut changes = vec![];
        if let Some(stashed) = self.stash.take() {
            changes.push(Change::NoChange(stashed));
        }

        while let Some(change) = self.iter.next() {
            match change {
                Change::NoChange(common_sequence) => {
                    if changes.is_empty() {
                        if self.iter.peek().is_some() {
                            changes.push(Change::NoChange(
                                common_sequence.starts_trimmed(self.context),
                            ));
                        }
                    } else if self.iter.peek().is_none() {
                        changes.push(Change::NoChange(common_sequence.ends_trimmed(self.context)));
                        break;
                    } else if let Some((head, tail)) = common_sequence.split(self.context) {
                        self.stash = Some(tail);
                        changes.push(Change::NoChange(head));
                        break;
                    } else {
                        changes.push(*change);
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

impl<'a, T: PartialEq + Eq + Clone + std::hash::Hash + Sync> Changes<'a, T> {
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

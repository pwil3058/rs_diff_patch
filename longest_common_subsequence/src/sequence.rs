// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use crate::range::Range;
use std::ops::Deref;

/// A sequence of items of type T
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Seq<T: PartialEq + Clone>(pub Box<[T]>);

impl<T: PartialEq + Clone> Deref for Seq<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: PartialEq + Clone> Seq<T> {
    /// Returns the `Range` from the index `from` to the end of the sequence
    pub fn range_from(&self, from: usize) -> Range {
        Range(from, self.len())
    }

    /// Returns an iterator over the items in `range`
    pub fn subsequence(&self, range: Range) -> impl DoubleEndedIterator<Item = &T> {
        self.0[range.0..range.1].iter()
    }

    /// Returns `true` if the given subsequence exists at the given index,
    pub fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool {
        if let Some(end) = at.checked_add(subsequence.len()) {
            end <= self.len() && self.0[at..end] == *subsequence
        } else {
            false // Arithmetic overflowed usize capacity
        }
    }
}

impl<T: PartialEq + Clone> From<&[T]> for Seq<T> {
    fn from(slice: &[T]) -> Self {
        Seq(slice.to_vec().into_boxed_slice())
    }
}

impl<T: PartialEq + Clone> From<Vec<T>> for Seq<T> {
    fn from(vec: Vec<T>) -> Self {
        Seq(vec.into_boxed_slice())
    }
}

impl<T: PartialEq + Clone> FromIterator<T> for Seq<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Seq(iter.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }
}

impl<'a, T: PartialEq + Clone> FromIterator<&'a T> for Seq<T> {
    fn from_iter<I: IntoIterator<Item = &'a T>>(iter: I) -> Self {
        Seq(iter
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
            .into_boxed_slice())
    }
}

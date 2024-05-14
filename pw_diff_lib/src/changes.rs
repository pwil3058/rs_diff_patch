// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::common_subsequence::CommonSubsequence;
use crate::range::{Len, Range};
use crate::sequence::{
    ByteItemIndices, ByteSequence, ContentItemIndices, Sequence, StringItemIndices, StringSequence,
};
use rayon::prelude::ParallelSliceMut;
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Change {
    NoChange(CommonSubsequence),
    Delete(Range, usize),
    Insert(usize, Range),
    Replace(Range, Range),
}

#[derive(Debug)]
pub struct ChangesGenerator<'a, T: PartialEq, S: Sequence<T>, I: ContentItemIndices<T>> {
    before: &'a S,
    after: &'a S,
    before_content_indices: I,
    phantom_data: PhantomData<&'a T>,
}

impl<'a> ChangesGenerator<'a, String, StringSequence, StringItemIndices> {
    pub fn new(before: &'a StringSequence, after: &'a StringSequence) -> Self {
        let before_content_indices = StringItemIndices::new(before.clone());
        Self {
            before,
            after,
            before_content_indices,
            phantom_data: PhantomData,
        }
    }
}

impl<'a> ChangesGenerator<'a, u8, ByteSequence, ByteItemIndices> {
    pub fn new(before: &'a ByteSequence, after: &'a ByteSequence) -> Self {
        let before_content_indices = ByteItemIndices::new(before.clone());
        Self {
            before,
            after,
            before_content_indices,
            phantom_data: PhantomData,
        }
    }
}

impl<'a, T: PartialEq, S: Sequence<T> + Clone, I: ContentItemIndices<T>>
    ChangesGenerator<'a, T, S, I>
{
    // pub fn new(before: &'a S, after: &'a S) -> Self {
    //     // I assume clone() just clones the pointer
    //     let before_content_indices = ContentItemIndices::<T>::new(before.clone());
    //     Self {
    //         before,
    //         after,
    //         before_content_indices,
    //         phantom_data: PhantomData,
    //     }
    // }

    fn longest_common_subsequence(
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
    /// use pw_diff_lib::changes::{ChangesGenerator};
    /// use pw_diff_lib::range::Range;
    /// use pw_diff_lib::common_subsequence::CommonSubsequence;
    /// use pw_diff_lib::sequence::{ContentItemIndices, StringItemIndices, StringSequence};
    /// use pw_diff_lib::changes::Change::*;
    /// let before_lines = StringSequence::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let after_lines = StringSequence::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let change_list = ChangesGenerator::<String, StringSequence, StringItemIndices>::new(&before_lines, &after_lines).generate();
    /// assert_eq!(
    ///     vec![
    ///         NoChange(CommonSubsequence(0,0,1)), Delete(Range(1, 2), 1),
    ///         NoChange(CommonSubsequence(2, 1, 2)), Replace(Range(4, 6), Range(3, 5)),
    ///         NoChange(CommonSubsequence(6, 5, 5)), Insert(11, Range(10, 11)),
    ///         NoChange(CommonSubsequence(11, 11, 2))
    ///     ],
    ///     change_list
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
            changes.push(Change::Delete(self.before.range_from(i), self.after.len()));
        } else if j < self.after.len() {
            changes.push(Change::Insert(self.before.len(), self.after.range_from(j)));
        }

        changes
    }
}

#[derive(Debug, Default)]
pub struct Changes<'a, T: PartialEq + 'a, S: Sequence<T>> {
    pub before: S,
    pub after: S,
    pub changes: Vec<Change>,
    phantom_data: PhantomData<&'a T>,
}

impl Changes<'_, String, StringSequence> {
    pub fn new(before: StringSequence, after: StringSequence) -> Self {
        let changes =
            ChangesGenerator::<String, StringSequence, StringItemIndices>::new(&before, &after)
                .generate();
        Self {
            before,
            after,
            changes,
            phantom_data: PhantomData,
        }
    }
}

impl Changes<'_, u8, ByteSequence> {
    pub fn new(before: ByteSequence, after: ByteSequence) -> Self {
        let changes =
            ChangesGenerator::<u8, ByteSequence, ByteItemIndices>::new(&before, &after).generate();
        Self {
            before,
            after,
            changes,
            phantom_data: PhantomData,
        }
    }
}

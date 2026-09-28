// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use crate::common_subsequence::CommonSubsequence;
use crate::range::{Len, Range};
use crate::sequence::Seq;
use std::collections::HashMap;

use rayon::prelude::ParallelSliceMut;

pub(crate) struct Data<'a, T: PartialEq + Eq + Clone + std::hash::Hash> {
    left: &'a Seq<T>,
    right: &'a Seq<T>,
    left_item_indices: HashMap<T, Vec<usize>>,
}

impl<'a, T: PartialEq + Eq + Clone + std::hash::Hash> Data<'a, T> {
    pub(crate) fn new(left: &'a Seq<T>, right: &'a Seq<T>) -> Self {
        let mut left_item_indices: HashMap<T, Vec<usize>> = HashMap::new();
        for (index, item) in left.iter().enumerate() {
            // Idiomatic Rust entry API avoids double lookup
            left_item_indices
                .entry(item.clone())
                .or_default()
                .push(index);
        }
        Data {
            left,
            right,
            left_item_indices,
        }
    }

    pub(crate) fn longest_common_subsequence(
        &self,
        left_range: Range,
        right_range: Range,
    ) -> Option<CommonSubsequence> {
        let mut best_lcs = CommonSubsequence::default();

        // Track lengths and the exact outer-loop index 'i' where the match happened.
        // This stops stale matching sequences from bleeding into later iterations.
        let mut lengths = vec![0usize; left_range.end()];
        let mut last_updated_at_i = vec![None; left_range.end()];

        for (i, item) in self.right.subsequence(right_range).enumerate() {
            let index = i + right_range.start();

            if let Some(indices) = self.left_item_indices.get(item) {
                // Loop backwards to safely update arrays in place without overriding the current step's dependency layout
                for &j in indices.iter().rev() {
                    if j < left_range.start() {
                        break;
                    }
                    if j >= left_range.end() {
                        continue;
                    }

                    // To be a continuous sequence match, the element at j - 1 must have matched
                    // in the immediate previous iteration (i - 1)
                    let k = if j > 0 && last_updated_at_i[j - 1] == Some(i.wrapping_sub(1)) {
                        lengths[j - 1] + 1
                    } else {
                        1
                    };

                    lengths[j] = k;
                    last_updated_at_i[j] = Some(i);

                    if k > best_lcs.len() {
                        best_lcs = CommonSubsequence(j + 1 - k, index + 1 - k, k);
                    }
                }
            }
        }

        if best_lcs.is_empty() {
            None
        } else {
            let count = self
                .left
                .subsequence(Range(left_range.start(), best_lcs.left_start()))
                .rev()
                .zip(
                    self.right
                        .subsequence(Range(right_range.start(), best_lcs.right_start()))
                        .rev(),
                )
                .take_while(|(a, b)| a == b)
                .count();
            best_lcs.incr_size_moving_starts(
                count.min(best_lcs.left_start()).min(best_lcs.right_start()),
            );

            if best_lcs.left_end() + 1 < left_range.end()
                && best_lcs.right_end() + 1 < right_range.end()
            {
                let count = self
                    .left
                    .subsequence(Range(best_lcs.left_end() + 1, left_range.end()))
                    .zip(
                        self.right
                            .subsequence(Range(best_lcs.right_end() + 1, right_range.end())),
                    )
                    .take_while(|(a, b)| a == b)
                    .count();
                best_lcs.incr_size_moving_ends(count);
            }

            Some(best_lcs)
        }
    }

    pub(crate) fn longest_common_subsequences(&self) -> Vec<CommonSubsequence> {
        let mut lifo = vec![(self.left.range_from(0), self.right.range_from(0))];
        let mut raw_lcses = vec![];
        while let Some((left_range, right_range)) = lifo.pop() {
            if let Some(lcs) = self.longest_common_subsequence(left_range, right_range) {
                if left_range.start() < lcs.left_start() && right_range.start() < lcs.right_start()
                {
                    lifo.push((
                        Range(left_range.start(), lcs.left_start()),
                        Range(right_range.start(), lcs.right_start()),
                    ))
                };
                if lcs.left_end() < left_range.end() && lcs.right_end() < right_range.end() {
                    lifo.push((
                        Range(lcs.left_end(), left_range.end()),
                        Range(lcs.right_end(), right_range.end()),
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
                if new_lcs.left_end() == lcs.left_start()
                    && new_lcs.right_end() == lcs.right_start()
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
}

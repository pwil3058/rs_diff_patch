// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use crate::common_subsequence::CommonSubsequence;
use crate::range::{Len, Range};
use crate::sequence::Seq;
use std::collections::HashMap;

use rayon::prelude::ParallelSliceMut;

pub(crate) struct Data<'a, T: PartialEq + Eq + Clone + std::hash::Hash + Sync> {
    left: &'a Seq<T>,
    right: &'a Seq<T>,
    left_item_indices: HashMap<T, Vec<usize>>,
}

impl<'a, T: PartialEq + Eq + Clone + std::hash::Hash + Sync> Data<'a, T> {
    pub(crate) fn new(left: &'a Seq<T>, right: &'a Seq<T>) -> Self {
        let mut left_item_indices: HashMap<T, Vec<usize>> = HashMap::new();
        for (index, item) in left.iter().enumerate() {
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
}

impl<'a, T: PartialEq + Eq + Clone + std::hash::Hash + Sync> Data<'a, T> {
    pub(crate) fn longest_common_subsequence(
        &self,
        left_range: Range,
        right_range: Range,
    ) -> Option<CommonSubsequence> {
        let mut best_lcs = CommonSubsequence::default();

        let mut j_to_len = HashMap::<isize, usize>::with_capacity(32);
        let mut new_j_to_len = HashMap::<isize, usize>::with_capacity(32);

        for (i, item) in self.right.subsequence(right_range).enumerate() {
            let index = i + right_range.start();

            new_j_to_len.clear();

            if let Some(indices) = self.left_item_indices.get(item) {
                for j in indices {
                    if j < &left_range.start() {
                        continue;
                    }
                    if j >= &left_range.end() {
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

            std::mem::swap(&mut j_to_len, &mut new_j_to_len);
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
        let mut raw_lcses =
            self.find_lcs_parallel(self.left.range_from(0), self.right.range_from(0));

        raw_lcses.par_sort();

        let mut lcses = Vec::with_capacity(raw_lcses.len());
        let mut iter = raw_lcses.into_iter();

        if let Some(mut current) = iter.next() {
            for next in iter {
                if current.left_end() == next.left_start()
                    && current.right_end() == next.right_start()
                {
                    current.incr_size_moving_ends(next.len());
                } else {
                    lcses.push(current);
                    current = next;
                }
            }
            lcses.push(current);
        }

        lcses
    }

    fn find_lcs_parallel(&self, left_range: Range, right_range: Range) -> Vec<CommonSubsequence> {
        if let Some(lcs) = self.longest_common_subsequence(left_range, right_range) {
            let mut results = vec![lcs];

            let has_left =
                left_range.start() < lcs.left_start() && right_range.start() < lcs.right_start();
            let has_right =
                lcs.left_end() < left_range.end() && lcs.right_end() < right_range.end();

            match (has_left, has_right) {
                (true, true) => {
                    let (mut left_lcses, mut right_lcses) = rayon::join(
                        || {
                            self.find_lcs_parallel(
                                Range(left_range.start(), lcs.left_start()),
                                Range(right_range.start(), lcs.right_start()),
                            )
                        },
                        || {
                            self.find_lcs_parallel(
                                Range(lcs.left_end(), left_range.end()),
                                Range(lcs.right_end(), right_range.end()),
                            )
                        },
                    );

                    results.append(&mut left_lcses);
                    results.append(&mut right_lcses);
                }
                (true, false) => {
                    let mut left_lcses = self.find_lcs_parallel(
                        Range(left_range.start(), lcs.left_start()),
                        Range(right_range.start(), lcs.right_start()),
                    );
                    results.append(&mut left_lcses);
                }
                (false, true) => {
                    let mut right_lcses = self.find_lcs_parallel(
                        Range(lcs.left_end(), left_range.end()),
                        Range(lcs.right_end(), right_range.end()),
                    );
                    results.append(&mut right_lcses);
                }
                (false, false) => {}
            }

            results
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod lcs_tests {
    use crate::common_subsequence::CommonSubsequence;
    use crate::lcs::Data;
    use crate::range::Range;
    use crate::sequence::Seq;

    #[test]
    fn lcs() {
        let left = Seq(vec![1, 2, 3, 4, 6, 7, 8, 9, 10].into_boxed_slice());
        let right = Seq(vec![1, 2, 3, 4, 6, 7, 8, 9, 10].into_boxed_slice());
        let data = Data::new(&left, &right);
        assert_eq!(
            data.longest_common_subsequence(Range(0, 8), Range(0, 8)),
            Some(CommonSubsequence(0, 0, 8))
        );
        assert_eq!(
            data.longest_common_subsequence(Range(0, 7), Range(3, 8)),
            Some(CommonSubsequence(3, 3, 4))
        );
    }
}

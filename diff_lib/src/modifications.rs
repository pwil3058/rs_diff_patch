// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use rayon::prelude::ParallelSliceMut;
use std::collections::HashMap;
use std::ops::RangeBounds;

use crate::crange::{CRange, Len};
use crate::lcs::LongestCommonSubsequence;
use crate::lines::BasicLines;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Modification {
    NoChange(LongestCommonSubsequence),
    Delete(CRange, usize),
    Insert(usize, CRange),
    Replace(CRange, CRange),
}

pub trait MapKey {
    fn map_key(&self) -> Vec<u8>;
}

impl MapKey for str {
    fn map_key(&self) -> Vec<u8> {
        //self.as_bytes().to_vec()
        crypto_hash::digest(crypto_hash::Algorithm::SHA1, &self.as_bytes())
    }
}

#[derive(Debug)]
pub struct ModGenerator<'a, A: BasicLines, P: BasicLines> {
    antemod: &'a A,
    postmod: &'a P,
    postmod_line_indices: HashMap<Vec<u8>, Vec<usize>>,
}

impl<'a, A: BasicLines, P: BasicLines> ModGenerator<'a, A, P> {
    pub fn new(antemod: &'a A, postmod: &'a P) -> Self {
        let mut postmod_line_indices = HashMap::<Vec<u8>, Vec<usize>>::new();
        for (index, line) in postmod.lines(..).enumerate() {
            let key = line.map_key();
            if let Some(vec) = postmod_line_indices.get_mut(&key) {
                vec.push(index);
            } else {
                postmod_line_indices.insert(key, vec![index]);
            }
        }

        Self {
            antemod,
            postmod,
            postmod_line_indices,
        }
    }

    fn longest_common_subsequence(
        &self,
        antemod_range_bounds: impl RangeBounds<usize>,
        postmod_range_bounds: impl RangeBounds<usize>,
    ) -> Option<LongestCommonSubsequence> {
        let antemod_range = CRange::from(antemod_range_bounds);
        let postmod_range = CRange::from(postmod_range_bounds);

        let mut best_lcs = LongestCommonSubsequence::default();

        let mut j_to_len = HashMap::<usize, usize>::new();
        for (i, line) in self.antemod.lines(antemod_range).enumerate() {
            let index = i + antemod_range.start();
            let mut new_j_to_len = HashMap::<usize, usize>::new();
            if let Some(indices) = self.postmod_line_indices.get(&line.map_key()) {
                for j in indices {
                    if j < &postmod_range.start() {
                        continue;
                    }
                    if j >= &postmod_range.end() {
                        break;
                    }

                    if j == &0 {
                        new_j_to_len.insert(0, 1);
                        if best_lcs.is_empty() {
                            best_lcs = LongestCommonSubsequence(index, 0, 1);
                        }
                    } else {
                        let k = match j_to_len.get(&(j - 1)) {
                            Some(k) => *k + 1,
                            None => 1,
                        };
                        new_j_to_len.insert(*j, k);
                        if k > best_lcs.len() {
                            best_lcs = LongestCommonSubsequence(index + 1 - k, j + 1 - k, k);
                        }
                    }
                }
            }
            j_to_len = new_j_to_len;
        }

        if best_lcs.is_empty() {
            None
        } else {
            let count = self
                .antemod
                .lines_reversed(antemod_range.start()..best_lcs.antemod_start())
                .zip(
                    self.postmod
                        .lines_reversed(postmod_range.start()..best_lcs.postmod_start()),
                )
                .take_while(|(a, b)| a == b)
                .count();
            best_lcs.decr_starts(count);

            let count = self
                .antemod
                .lines(best_lcs.antemod_end() + 1..antemod_range.end())
                .zip(
                    self.postmod
                        .lines(best_lcs.postmod_end() + 1..postmod_range.end()),
                )
                .take_while(|(a, b)| a == b)
                .count();
            best_lcs.incr_size(count);

            Some(best_lcs)
        }
    }

    fn longest_common_subsequences(&self) -> Vec<LongestCommonSubsequence> {
        let mut lifo = vec![(CRange(0, self.antemod.len()), CRange(0, self.postmod.len()))];
        let mut raw_lcses = vec![];
        while let Some((antemod_range, postmod_range)) = lifo.pop() {
            if let Some(lcs) =
                self.longest_common_subsequence(antemod_range.clone(), postmod_range.clone())
            {
                if antemod_range.start() < lcs.antemod_start()
                    && postmod_range.start() < lcs.postmod_start()
                {
                    lifo.push((
                        CRange(antemod_range.start(), lcs.antemod_start()),
                        CRange(postmod_range.start(), lcs.postmod_start()),
                    ))
                };
                if lcs.antemod_end() < antemod_range.end()
                    && lcs.postmod_end() < postmod_range.end()
                {
                    lifo.push((
                        CRange(lcs.antemod_end(), antemod_range.end()),
                        CRange(lcs.postmod_end(), postmod_range.end()),
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
                if new_lcs.antemod_end() == lcs.antemod_start()
                    && new_lcs.postmod_end() == lcs.postmod_start()
                {
                    new_lcs.incr_size(lcs.len());
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
    /// use diff_lib::crange::CRange;
    /// use diff_lib::lines::LazyLines;
    /// use diff_lib::lcs::LongestCommonSubsequence;
    /// use diff_lib::modifications::ModGenerator;
    /// use diff_lib::modifications::Modification::*;
    ///
    /// let ante_lines = LazyLines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let post_lines = LazyLines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let modlist = ModGenerator::new(&ante_lines, &post_lines).generate();
    /// assert_eq!(
    ///     vec![
    ///         NoChange(LongestCommonSubsequence(0,0,1)), Delete(CRange(1, 2), 1),     ///
    ///         NoChange(LongestCommonSubsequence(2, 1, 2)), Replace(CRange(4, 6), CRange(3, 5)),
    ///         NoChange(LongestCommonSubsequence(6, 5, 5)), Insert(11, CRange(10, 11)),
    ///         NoChange(LongestCommonSubsequence(11, 11, 2))
    ///     ],
    ///     modlist
    /// );
    /// ```
    pub fn generate(&self) -> Vec<Modification> {
        let mut op_codes = vec![];
        let mut i = 0usize;
        let mut j = 0usize;

        for lcs in self.longest_common_subsequences() {
            if i < lcs.antemod_start() && j < lcs.postmod_start() {
                op_codes.push(Modification::Replace(
                    CRange(i, lcs.antemod_start()),
                    CRange(j, lcs.postmod_start()),
                ));
            } else if i < lcs.antemod_start() {
                op_codes.push(Modification::Delete(
                    CRange(i, lcs.antemod_start()),
                    lcs.postmod_start(),
                ));
            } else if j < lcs.postmod_start() {
                op_codes.push(Modification::Insert(
                    lcs.antemod_start(),
                    CRange(j, lcs.postmod_start()),
                ));
            }
            op_codes.push(Modification::NoChange(lcs));
            i = lcs.antemod_end();
            j = lcs.postmod_end();
        }
        if i < self.antemod.len() && j < self.postmod.len() {
            op_codes.push(Modification::Replace(
                CRange(i, self.antemod.len()),
                CRange(j, self.postmod.len()),
            ));
        } else if i < self.antemod.len() {
            op_codes.push(Modification::Delete(
                CRange(i, self.antemod.len()),
                self.postmod.len(),
            ));
        } else if j < self.postmod.len() {
            op_codes.push(Modification::Insert(
                self.antemod.len(),
                CRange(j, self.postmod.len()),
            ));
        }

        op_codes
    }
}

// #[derive(Debug, Default)]
// pub struct Mods<A: BasicLines, P: BasicLines> {
//     antemod: A,
//     postmod: P,
//     mods: Vec<Mod>,
// }
//
// impl<A: BasicLines, P: BasicLines> Mods<A, P> {
//     pub fn new(antemod: A, postmod: P) -> Self {
//         let mods = ModGenerator::new(&antemod, &postmod).generate();
//         Self {
//             antemod,
//             postmod,
//             mods,
//         }
//     }
// }

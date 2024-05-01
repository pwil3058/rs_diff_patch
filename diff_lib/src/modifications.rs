// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use rayon::prelude::ParallelSliceMut;
use std::collections::HashMap;
use std::iter::Peekable;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::slice::Iter;

use crate::lcs::CommonSubsequence;
use crate::lines::{BasicLines, DiffableLines};
use crate::range::{Len, Range};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Modification {
    NoChange(CommonSubsequence),
    Delete(Range, usize),
    Insert(usize, Range),
    Replace(Range, Range),
}

#[derive(Debug)]
pub struct ModGenerator<'a, A: BasicLines, P: BasicLines> {
    before: &'a A,
    after: &'a P,
    after_line_indices: HashMap<String, Vec<usize>>,
}

impl<'a, A: BasicLines, P: BasicLines> ModGenerator<'a, A, P> {
    pub fn new(before: &'a A, after: &'a P) -> Self {
        let mut after_line_indices = HashMap::<String, Vec<usize>>::new();
        for (index, line) in after.lines(after.range_from(0)).enumerate() {
            if let Some(vec) = after_line_indices.get_mut(line) {
                vec.push(index)
            } else {
                after_line_indices.insert(line.to_string(), vec![index]);
            }
        }

        Self {
            before,
            after,
            after_line_indices,
        }
    }

    fn longest_common_subsequence(
        &self,
        before_range: Range,
        after_range: Range,
    ) -> Option<CommonSubsequence> {
        let mut best_lcs = CommonSubsequence::default();

        let mut j_to_len = HashMap::<usize, usize>::new();
        for (i, line) in self.before.lines(before_range).enumerate() {
            let index = i + before_range.start();
            let mut new_j_to_len = HashMap::<usize, usize>::new();
            if let Some(indices) = self.after_line_indices.get(line) {
                for j in indices {
                    if j < &after_range.start() {
                        continue;
                    }
                    if j >= &after_range.end() {
                        break;
                    }

                    if j == &0 {
                        new_j_to_len.insert(0, 1);
                        if best_lcs.is_empty() {
                            best_lcs = CommonSubsequence(index, 0, 1);
                        }
                    } else {
                        let k = match j_to_len.get(&(j - 1)) {
                            Some(k) => *k + 1,
                            None => 1,
                        };
                        new_j_to_len.insert(*j, k);
                        if k > best_lcs.len() {
                            best_lcs = CommonSubsequence(index + 1 - k, j + 1 - k, k);
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
                .before
                .lines(Range(before_range.start(), best_lcs.before_start()))
                .rev()
                .zip(
                    self.after
                        .lines(Range(after_range.start(), best_lcs.after_start()))
                        .rev(),
                )
                .take_while(|(a, b)| a == b)
                .count();
            best_lcs.decr_starts(
                count
                    .min(best_lcs.before_start())
                    .min(best_lcs.after_start()),
            );

            if best_lcs.before_end() + 1 < before_range.end()
                && best_lcs.after_end() + 1 < after_range.end()
            {
                let count = self
                    .before
                    .lines(Range(best_lcs.before_end() + 1, before_range.end()))
                    .zip(
                        self.after
                            .lines(Range(best_lcs.after_end() + 1, after_range.end())),
                    )
                    .take_while(|(a, b)| a == b)
                    .count();
                best_lcs.incr_size(count);
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
    /// use diff_lib::range::Range;
    /// use diff_lib::lines::Lines;
    /// use diff_lib::lcs::CommonSubsequence;
    /// use diff_lib::modifications::ModGenerator;
    /// use diff_lib::modifications::Modification::*;
    ///
    /// let before_lines = Lines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let after_lines = Lines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let modlist = ModGenerator::new(&before_lines, &after_lines).generate();
    /// assert_eq!(
    ///     vec![
    ///         NoChange(CommonSubsequence(0,0,1)), Delete(Range(1, 2), 1),     ///
    ///         NoChange(CommonSubsequence(2, 1, 2)), Replace(Range(4, 6), Range(3, 5)),
    ///         NoChange(CommonSubsequence(6, 5, 5)), Insert(11, Range(10, 11)),
    ///         NoChange(CommonSubsequence(11, 11, 2))
    ///     ],
    ///     modlist
    /// );
    /// ```
    pub fn generate(&self) -> Vec<Modification> {
        let mut op_codes = vec![];
        let mut i = 0usize;
        let mut j = 0usize;

        for lcs in self.longest_common_subsequences() {
            if i < lcs.before_start() && j < lcs.after_start() {
                op_codes.push(Modification::Replace(
                    Range(i, lcs.before_start()),
                    Range(j, lcs.after_start()),
                ));
            } else if i < lcs.before_start() {
                op_codes.push(Modification::Delete(
                    Range(i, lcs.before_start()),
                    lcs.after_start(),
                ));
            } else if j < lcs.after_start() {
                op_codes.push(Modification::Insert(
                    lcs.before_start(),
                    Range(j, lcs.after_start()),
                ));
            }
            op_codes.push(Modification::NoChange(lcs));
            i = lcs.before_end();
            j = lcs.after_end();
        }
        if i < self.before.len() && j < self.after.len() {
            op_codes.push(Modification::Replace(
                self.before.range_from(i),
                self.after.range_from(j),
            ));
        } else if i < self.before.len() {
            op_codes.push(Modification::Delete(
                self.before.range_from(i),
                self.after.len(),
            ));
        } else if j < self.after.len() {
            op_codes.push(Modification::Insert(
                self.before.len(),
                self.after.range_from(j),
            ));
        }

        op_codes
    }
}

#[derive(Debug, Default)]
pub struct Modifications<A: BasicLines, P: BasicLines> {
    before: A,
    after: P,
    mods: Vec<Modification>,
}

impl<A: BasicLines, P: BasicLines> Modifications<A, P> {
    pub fn new(before: A, after: P) -> Self {
        let mods = ModGenerator::new(&before, &after).generate();
        Self {
            before,
            after,
            mods,
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct ModificationChunk(pub Vec<Modification>);

impl Deref for ModificationChunk {
    type Target = Vec<Modification>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ModificationChunk {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ModificationChunk {
    fn starts(&self) -> (usize, usize) {
        use Modification::*;
        if let Some(modn) = self.0.first() {
            match modn {
                Delete(range, after_start) => (range.start(), *after_start),
                NoChange(match_) => (match_.before_start(), match_.after_start()),
                Insert(before_start, after_range) => (*before_start, after_range.start()),
                Replace(before_range, after_range) => (before_range.start(), after_range.start()),
            }
        } else {
            (0, 0)
        }
    }

    fn ends(&self) -> (usize, usize) {
        use Modification::*;
        if let Some(op_code) = self.0.last() {
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

    pub fn context_lengths(&self) -> (usize, usize) {
        use Modification::NoChange;
        let start = if let Some(modn) = self.first() {
            match modn {
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
        (start, end)
    }
}

pub struct ModificationChunkIter<'a> {
    iter: Peekable<Iter<'a, Modification>>,
    context: usize,
    stash: Option<CommonSubsequence>,
}

impl<'a> Iterator for ModificationChunkIter<'a> {
    type Item = ModificationChunk;

    fn next(&mut self) -> Option<Self::Item> {
        use Modification::NoChange;
        let mut chunk = ModificationChunk::default();
        if let Some(stashed) = self.stash {
            chunk.push(NoChange(stashed));
            self.stash = None;
        }
        while let Some(modn) = self.iter.next() {
            match modn {
                NoChange(common_sequence) => {
                    if chunk.is_empty() {
                        if self.iter.peek().is_some() {
                            chunk.push(NoChange(common_sequence.starts_trimmed(self.context)));
                        }
                    } else if self.iter.peek().is_none() {
                        chunk.push(NoChange(common_sequence.ends_trimmed(self.context)));
                        break;
                    } else if let Some((head, tail)) = common_sequence.split(self.context) {
                        self.stash = Some(tail);
                        chunk.push(NoChange(head));
                        break;
                    } else {
                        chunk.push(*modn)
                    }
                }
                _ => {
                    chunk.push(*modn);
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

impl<A: BasicLines, P: BasicLines> Modifications<A, P> {
    /// Return an iterator over ModificationChunks generated with the given `context` size.
    ///
    /// Example:
    ///
    /// ```
    /// use diff_lib::lcs::CommonSubsequence;
    /// use diff_lib::lines::Lines;
    /// use diff_lib::modifications::{ModificationChunk, Modifications,Modification};
    /// use diff_lib::range::Range;
    /// use Modification::*;
    ///
    /// let before = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    /// let after = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    /// let modifications = Modifications::new(Lines::from(before), Lines::from(after));
    /// let modn_chunks: Vec<_> = modifications.modification_chunks(2).collect();
    /// assert_eq!(
    ///     modn_chunks,
    ///     vec![
    ///         ModificationChunk(vec![
    ///             NoChange(CommonSubsequence(0, 0, 1)),
    ///             Delete(Range(1, 2), 1),
    ///             NoChange(CommonSubsequence(2, 1, 2)),
    ///             Replace(Range(4, 6), Range(3, 5)),
    ///             NoChange(CommonSubsequence(6, 5, 2))
    ///         ]),
    ///         ModificationChunk(vec![
    ///             NoChange(CommonSubsequence(9, 8, 2)),
    ///             Insert(11, Range(10, 11)),
    ///             NoChange(CommonSubsequence(11, 11, 2))
    ///         ]),
    ///     ]
    /// );
    /// ```
    pub fn modification_chunks<'a>(&'a self, context: usize) -> ModificationChunkIter<'a> {
        ModificationChunkIter {
            iter: self.mods.iter().peekable(),
            context,
            stash: None,
        }
    }
}

pub struct ChunkIter<'a, A, P, I>
where
    A: DiffableLines,
    P: DiffableLines,
{
    pub before: &'a A,
    pub after: &'a P,
    pub iter: ModificationChunkIter<'a>,
    phantom_data: PhantomData<&'a I>,
}

impl<A: DiffableLines, P: DiffableLines> Modifications<A, P> {
    pub fn chunks<'a, I>(&'a self, context: usize) -> ChunkIter<'a, A, P, I> {
        ChunkIter {
            before: &self.before,
            after: &self.after,
            iter: self.modification_chunks(context),
            phantom_data: PhantomData::default(),
        }
    }
}

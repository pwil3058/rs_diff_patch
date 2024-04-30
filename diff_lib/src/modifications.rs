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
        for (index, line) in postmod.lines(postmod.range_from(0)).enumerate() {
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
        antemod_range: Range,
        postmod_range: Range,
    ) -> Option<CommonSubsequence> {
        let mut best_lcs = CommonSubsequence::default();

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
                .antemod
                .lines(Range(antemod_range.start(), best_lcs.antemod_start()))
                .rev()
                .zip(
                    self.postmod
                        .lines(Range(postmod_range.start(), best_lcs.postmod_start()))
                        .rev(),
                )
                .take_while(|(a, b)| a == b)
                .count();
            best_lcs.decr_starts(
                count
                    .min(best_lcs.antemod_start())
                    .min(best_lcs.postmod_start()),
            );

            if best_lcs.antemod_end() + 1 < antemod_range.end()
                && best_lcs.postmod_end() + 1 < postmod_range.end()
            {
                let count = self
                    .antemod
                    .lines(Range(best_lcs.antemod_end() + 1, antemod_range.end()))
                    .zip(
                        self.postmod
                            .lines(Range(best_lcs.postmod_end() + 1, postmod_range.end())),
                    )
                    .take_while(|(a, b)| a == b)
                    .count();
                best_lcs.incr_size(count);
            }

            Some(best_lcs)
        }
    }

    fn longest_common_subsequences(&self) -> Vec<CommonSubsequence> {
        let mut lifo = vec![(self.antemod.range_from(0), self.postmod.range_from(0))];
        let mut raw_lcses = vec![];
        while let Some((antemod_range, postmod_range)) = lifo.pop() {
            if let Some(lcs) = self.longest_common_subsequence(antemod_range, postmod_range) {
                if antemod_range.start() < lcs.antemod_start()
                    && postmod_range.start() < lcs.postmod_start()
                {
                    lifo.push((
                        Range(antemod_range.start(), lcs.antemod_start()),
                        Range(postmod_range.start(), lcs.postmod_start()),
                    ))
                };
                if lcs.antemod_end() < antemod_range.end()
                    && lcs.postmod_end() < postmod_range.end()
                {
                    lifo.push((
                        Range(lcs.antemod_end(), antemod_range.end()),
                        Range(lcs.postmod_end(), postmod_range.end()),
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
    /// use diff_lib::range::Range;
    /// use diff_lib::lines::Lines;
    /// use diff_lib::lcs::CommonSubsequence;
    /// use diff_lib::modifications::ModGenerator;
    /// use diff_lib::modifications::Modification::*;
    ///
    /// let ante_lines = Lines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n");
    /// let post_lines = Lines::from("A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n");
    /// let modlist = ModGenerator::new(&ante_lines, &post_lines).generate();
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
            if i < lcs.antemod_start() && j < lcs.postmod_start() {
                op_codes.push(Modification::Replace(
                    Range(i, lcs.antemod_start()),
                    Range(j, lcs.postmod_start()),
                ));
            } else if i < lcs.antemod_start() {
                op_codes.push(Modification::Delete(
                    Range(i, lcs.antemod_start()),
                    lcs.postmod_start(),
                ));
            } else if j < lcs.postmod_start() {
                op_codes.push(Modification::Insert(
                    lcs.antemod_start(),
                    Range(j, lcs.postmod_start()),
                ));
            }
            op_codes.push(Modification::NoChange(lcs));
            i = lcs.antemod_end();
            j = lcs.postmod_end();
        }
        if i < self.antemod.len() && j < self.postmod.len() {
            op_codes.push(Modification::Replace(
                self.antemod.range_from(i),
                self.postmod.range_from(j),
            ));
        } else if i < self.antemod.len() {
            op_codes.push(Modification::Delete(
                self.antemod.range_from(i),
                self.postmod.len(),
            ));
        } else if j < self.postmod.len() {
            op_codes.push(Modification::Insert(
                self.antemod.len(),
                self.postmod.range_from(j),
            ));
        }

        op_codes
    }
}

#[derive(Debug, Default)]
pub struct Modifications<A: BasicLines, P: BasicLines> {
    antemod: A,
    postmod: P,
    mods: Vec<Modification>,
}

impl<A: BasicLines, P: BasicLines> Modifications<A, P> {
    pub fn new(antemod: A, postmod: P) -> Self {
        let mods = ModGenerator::new(&antemod, &postmod).generate();
        Self {
            antemod,
            postmod,
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
                Delete(range, postmod_start) => (range.start(), *postmod_start),
                NoChange(match_) => (match_.antemod_start(), match_.postmod_start()),
                Insert(antemod_start, postmod_range) => (*antemod_start, postmod_range.start()),
                Replace(antemod_range, postmod_range) => {
                    (antemod_range.start(), postmod_range.start())
                }
            }
        } else {
            (0, 0)
        }
    }

    fn ends(&self) -> (usize, usize) {
        use Modification::*;
        if let Some(op_code) = self.0.last() {
            match op_code {
                Delete(range, postmod_start) => (range.end(), *postmod_start),
                NoChange(match_) => (match_.antemod_end(), match_.postmod_end()),
                Insert(antemod_start, postmod_range) => (*antemod_start, postmod_range.end()),
                Replace(antemod_range, postmod_range) => (antemod_range.end(), postmod_range.end()),
            }
        } else {
            (0, 0)
        }
    }

    pub fn ranges(&self) -> (Range, Range) {
        let (antemod_start, postmod_start) = self.starts();
        let (antemod_end, postmod_end) = self.ends();

        (
            Range(antemod_start, antemod_end),
            Range(postmod_start, postmod_end),
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
    pub antemod: &'a A,
    pub postmod: &'a P,
    pub iter: ModificationChunkIter<'a>,
    phantom_data: PhantomData<&'a I>,
}

impl<A: DiffableLines, P: DiffableLines> Modifications<A, P> {
    pub fn chunks<'a, I>(&'a self, context: usize) -> ChunkIter<'a, A, P, I> {
        ChunkIter {
            antemod: &self.antemod,
            postmod: &self.postmod,
            iter: self.modification_chunks(context),
            phantom_data: PhantomData::default(),
        }
    }
}

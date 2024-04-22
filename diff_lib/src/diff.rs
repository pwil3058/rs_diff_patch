// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::{Applies, ApplyChunkInto, MatchesAt, ProgressData};
use crate::lines::BasicLines;
use crate::modifications::ChunkIter;
use crate::snippet::Snippet;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffChunk {
    context_lengths: (usize, usize),
    antemodn: Snippet,
    postmodn: Snippet,
}

impl<'a, A: BasicLines, P: BasicLines> Iterator for ChunkIter<'a, A, P, DiffChunk> {
    type Item = DiffChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let modn_chunk = self.iter.next()?;
        let (antemodn_range, postmodn_range) = modn_chunk.ranges();
        let context_lengths = modn_chunk.context_lengths();
        let antemodn = Snippet {
            start: antemodn_range.start(),
            lines: self
                .antemod
                .lines(antemodn_range)
                .map(|l| l.to_string())
                .collect(),
        };
        let postmodn = Snippet {
            start: postmodn_range.start(),
            lines: self
                .postmod
                .lines(postmodn_range)
                .map(|l| l.to_string())
                .collect(),
        };

        Some(DiffChunk {
            context_lengths,
            antemodn,
            postmodn,
        })
    }
}

impl DiffChunk {
    pub fn antemodn(&self, reverse: bool) -> &Snippet {
        if reverse {
            &self.postmodn
        } else {
            &self.antemodn
        }
    }

    pub fn postmodn(&self, reverse: bool) -> &Snippet {
        if reverse {
            &self.antemodn
        } else {
            &self.postmodn
        }
    }
}

impl ApplyChunkInto for DiffChunk {
    fn antemodn_lines(
        &self,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> impl Iterator<Item = &String> {
        if reverse {
            self.postmodn.lines(reductions)
        } else {
            self.antemodn.lines(reductions)
        }
    }

    fn postmodn_lines(
        &self,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> impl Iterator<Item = &String> {
        if reverse {
            self.antemodn.lines(reductions)
        } else {
            self.postmodn.lines(reductions)
        }
    }

    fn applies(&self, lines: &impl MatchesAt, offset: isize, reverse: bool) -> Option<Applies> {
        let antemodn = self.antemodn(reverse);
        let start = antemodn.start as isize + offset;
        if !start.is_negative() && lines.matches_at(&antemodn.lines, start as usize) {
            Some(Applies::Cleanly)
        } else {
            let max_reduction = self.context_lengths.0.max(self.context_lengths.1);
            for redn in 1..max_reduction {
                let start_redn = redn.min(self.context_lengths.0);
                let end_redn = redn.min(self.context_lengths.1);
                let adj_start = start + start_redn as isize;
                if !adj_start.is_negative()
                    && lines.matches_at(
                        &antemodn.lines[start_redn..antemodn.length(None) - end_redn],
                        adj_start as usize,
                    )
                {
                    return Some(Applies::WithReductions((start_redn, end_redn)));
                }
            }
            None
        }
    }

    fn applies_nearby(
        &self,
        lines: &impl MatchesAt,
        not_before: usize,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, Applies)> {
        let antemodn = self.antemodn(reverse);
        let not_after = if let Some(next_chunk) = next_chunk {
            let next_chunk_before = if reverse {
                &next_chunk.postmodn
            } else {
                &next_chunk.antemodn
            };
            next_chunk_before
                .start
                .checked_add_signed(offset)
                .expect("overflow")
                - antemodn.length(Some(self.context_lengths))
        } else {
            lines.len() - antemodn.length(Some(self.context_lengths))
        };
        let mut backward_done = false;
        let mut forward_done = false;
        for i in 1isize.. {
            if !backward_done {
                let adjusted_offset = offset - i;
                if antemodn.start as isize + adjusted_offset < not_before as isize {
                    backward_done = true;
                } else {
                    if let Some(applies) = self.applies(lines, adjusted_offset, reverse) {
                        return Some((-i, applies));
                    }
                }
            }
            if !forward_done {
                let adjusted_offset = offset + i;
                if antemodn.start as isize + adjusted_offset < not_after as isize {
                    if let Some(applies) = self.applies(lines, adjusted_offset, reverse) {
                        return Some((i, applies));
                    }
                } else {
                    forward_done = true
                }
            }
            if forward_done && backward_done {
                break;
            }
        }
        None
    }

    fn apply_into<'a, L, W>(
        &self,
        pd: &mut ProgressData<'a, L>,
        into: &mut W,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> std::io::Result<()>
    where
        L: BasicLines,
        W: Write,
    {
        let eol = pd.lines.eol();
        let antemodn = self.antemodn(reverse);
        let end = antemodn.start(pd.offset, reductions);
        for line in pd.lines.lines(pd.consumed..end) {
            into.write_all(line.as_bytes())?;
            into.write_all(eol.as_bytes())?;
        }
        for line in self.postmodn(reverse).lines(reductions) {
            into.write_all(line.as_bytes())?;
            into.write_all(eol.as_bytes())?;
        }
        pd.consumed = end + antemodn.length(reductions);
        Ok(())
    }

    fn already_applied_into<'a, L, W>(
        &self,
        pd: &mut ProgressData<'a, L>,
        into: &mut W,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> std::io::Result<()>
    where
        L: BasicLines,
        W: Write,
    {
        let eol = pd.lines.eol();
        let postmodn = self.postmodn(reverse);
        let end = postmodn.start(pd.offset, reductions) + postmodn.length(reductions);
        for line in pd.lines.lines(pd.consumed..end) {
            into.write_all(line.as_bytes())?;
            into.write_all(eol.as_bytes())?;
        }
        pd.consumed = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests;

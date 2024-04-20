// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::lines::BasicLines;
use crate::modifications::ChunkIter;
use crate::snippet::Snippet;

pub struct DiffChunk {
    context_lengths: (usize, usize),
    antemodn: Snippet,
    postmodn: Snippet,
}

impl<'a, A: BasicLines, P: BasicLines> Iterator for ChunkIter<'a, A, P, DiffChunk> {
    type Item = DiffChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let modn_chunk = self.iter.next()?;
        let (before_range, after_range) = modn_chunk.ranges();
        let context_lengths = modn_chunk.context_lengths();
        let antemodn = Snippet {
            start: before_range.start(),
            lines: self
                .antemod
                .lines(before_range)
                .map(|l| l.to_string())
                .collect(),
        };
        let postmodn = Snippet {
            start: after_range.start(),
            lines: self
                .postmod
                .lines(after_range)
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

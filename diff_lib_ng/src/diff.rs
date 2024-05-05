// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::{ApplyChunkClean, ApplyChunksClean, PatchableData, PatchableDataIfce};
use crate::data::{Data, DataIfce};
use crate::modifications::ChunkIter;
use crate::range::Len;
use crate::snippet::{Snippet, SnippetWrite};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

use crate::data::ExtractSnippet;

#[derive(Debug, Serialize, Deserialize)]
pub struct ByteChangeChunk {
    context_lengths: (u8, u8),
    before: Snippet<u8>,
    after: Snippet<u8>,
}

impl<'a> Iterator for ChunkIter<'a, u8>
where
    Data<u8>: ExtractSnippet<u8>,
{
    type Item = ByteChangeChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let modn_chunk = self.iter.next()?;
        let (before_range, after_range) = modn_chunk.ranges();

        Some(ByteChangeChunk {
            context_lengths: modn_chunk.context_lengths(),
            before: self.before.extract_snippet(before_range),
            after: self.after.extract_snippet(after_range),
        })
    }
}

impl ByteChangeChunk {
    pub fn before(&self, reverse: bool) -> &Snippet<u8> {
        if reverse {
            &self.after
        } else {
            &self.before
        }
    }

    pub fn after(&self, reverse: bool) -> &Snippet<u8> {
        if reverse {
            &self.before
        } else {
            &self.after
        }
    }
}

impl<'a> ApplyChunkClean<'a, u8, Data<u8>> for ByteChangeChunk {
    fn applies(&self, data: &Data<u8>, reverse: bool) -> bool {
        let before = self.before(reverse);
        data.has_subsequence_at(&before.items, before.start)
    }

    fn is_already_applied(&self, data: &Data<u8>, reverse: bool) -> bool {
        let after = self.after(reverse);
        data.has_subsequence_at(&after.items, after.start)
    }

    fn apply_into<W: io::Write>(
        &self,
        pd: &mut PatchableData<u8, Data<u8>>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool> {
        let before = self.before(reverse);
        if pd.write_upto_into(before.start, into)? {
            self.after(reverse).write_into(into, None)?;
            pd.advance_consumed_by(before.len());
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn already_applied_into<W: io::Write>(
        &self,
        pd: &mut PatchableData<u8, Data<u8>>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool> {
        let after = self.after(reverse);
        pd.write_upto_into(after.start + after.len(), into)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ByteChangeDiff {
    before_path: PathBuf,
    after_path: PathBuf,
    compressed: bool,
    chunks: Box<[ByteChangeChunk]>,
}

impl<'a> ApplyChunksClean<'a, u8, Data<u8>, ByteChangeChunk> for ByteChangeDiff {
    fn chunks<'b>(&'b self) -> impl Iterator<Item = &'b ByteChangeChunk>
    where
        ByteChangeChunk: 'b,
    {
        self.chunks.iter()
    }
}

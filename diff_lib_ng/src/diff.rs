// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::{
    Applies, ApplyChunkClean, ApplyChunkFuzzy, ApplyChunksClean, PatchableData, PatchableDataIfce,
};
use crate::data::{Data, DataIfce, WriteDataInto};
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

impl<'a> ApplyChunkClean<u8, Data<u8>> for ByteChangeChunk {
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
        if pd.write_into_upto(into, before.start)? {
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
        pd.write_into_upto(into, after.start + after.len())
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TextChangeChunk {
    context_lengths: (u8, u8),
    before: Snippet<String>,
    after: Snippet<String>,
}

impl<'a> Iterator for ChunkIter<'a, String>
where
    Data<String>: ExtractSnippet<String>,
{
    type Item = crate::diff::TextChangeChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let modn_chunk = self.iter.next()?;
        let (before_range, after_range) = modn_chunk.ranges();

        Some(crate::diff::TextChangeChunk {
            context_lengths: modn_chunk.context_lengths(),
            before: self.before.extract_snippet(before_range),
            after: self.after.extract_snippet(after_range),
        })
    }
}

impl crate::diff::TextChangeChunk {
    pub fn before(&self, reverse: bool) -> &Snippet<String> {
        if reverse {
            &self.after
        } else {
            &self.before
        }
    }

    pub fn after(&self, reverse: bool) -> &Snippet<String> {
        if reverse {
            &self.before
        } else {
            &self.after
        }
    }
}

impl ApplyChunkFuzzy<String, Data<String>> for TextChangeChunk {
    fn applies(&self, patchable: &Data<String>, offset: isize, reverse: bool) -> Option<Applies> {
        let before = self.before(reverse);
        let start = before.start as isize + offset;
        if !start.is_negative() && patchable.has_subsequence_at(&before.items, start as usize) {
            Some(Applies::Cleanly)
        } else {
            let max_reduction = self.context_lengths.0.max(self.context_lengths.1);
            for redn in 1..max_reduction {
                let start_redn = redn.min(self.context_lengths.0);
                let end_redn = redn.min(self.context_lengths.1);
                let adj_start = start + start_redn as isize;
                if !adj_start.is_negative()
                    && patchable.has_subsequence_at(
                        &before.items
                            [start_redn as usize..before.adj_length(None) - end_redn as usize],
                        adj_start as usize,
                    )
                {
                    return Some(Applies::WithReductions((start_redn, end_redn)));
                }
            }
            None
        }
    }

    fn apply_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut PatchableData<String, Data<String>>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        let before = self.before(reverse);
        let end = before.adj_start(offset, reductions);
        pd.write_into_upto(into, end)?;
        self.after(reverse).write_into(into, None)?;
        pd.advance_consumed_by(before.adj_length(reductions));
        Ok(())
    }

    fn applies_nearby(
        &self,
        pd: &PatchableData<String, Data<String>>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, Applies)> {
        None
    }

    fn is_already_applied(
        &self,
        patchable: &Data<String>,
        offset: isize,
        reverse: bool,
    ) -> Option<Applies> {
        None
    }

    fn is_already_applied_nearby(
        &self,
        pd: &PatchableData<String, Data<String>>,
        ext_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, Applies)> {
        None
    }

    fn already_applied_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut PatchableData<String, Data<String>>,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        Ok(())
    }

    fn write_failure_data_into<W: io::Write>(&self, into: &mut W) {}
}

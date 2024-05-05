// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::{
    ApplyChunkClean, ApplyChunkFuzzy, ApplyChunksClean, PatchableData, PatchableDataIfce, WillApply,
};
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

impl<'a> ApplyChunkClean<u8, Data<u8>> for ByteChangeChunk {
    fn will_apply(&self, data: &Data<u8>, reverse: bool) -> bool {
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
    fn will_apply(
        &self,
        patchable: &Data<String>,
        offset: isize,
        reverse: bool,
    ) -> Option<WillApply> {
        let before = self.before(reverse);
        let start = before.start as isize + offset;
        if !start.is_negative() && patchable.has_subsequence_at(&before.items, start as usize) {
            Some(WillApply::Cleanly)
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
                    return Some(WillApply::WithReductions((start_redn, end_redn)));
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

    fn will_apply_nearby(
        &self,
        pd: &PatchableData<String, Data<String>>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)> {
        let before = self.before(reverse);
        let not_after = if let Some(next_chunk) = next_chunk {
            let next_chunk_before = if reverse {
                &next_chunk.after
            } else {
                &next_chunk.before
            };
            next_chunk_before
                .start
                .checked_add_signed(offset)
                .expect("overflow")
                - before.adj_length(Some(self.context_lengths))
        } else {
            pd.data().len() - before.adj_length(Some(self.context_lengths))
        };
        let mut backward_done = false;
        let mut forward_done = false;
        for i in 1isize.. {
            if !backward_done {
                let adjusted_offset = offset - i;
                if before.start as isize + adjusted_offset < pd.consumed() as isize {
                    backward_done = true;
                } else {
                    if let Some(will_apply) = self.will_apply(pd.data(), adjusted_offset, reverse) {
                        return Some((-i, will_apply));
                    }
                }
            }
            if !forward_done {
                let adjusted_offset = offset + i;
                if before.start as isize + adjusted_offset < not_after as isize {
                    if let Some(will_apply) = self.will_apply(pd.data(), adjusted_offset, reverse) {
                        return Some((i, will_apply));
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

    fn is_already_applied(
        &self,
        patchable: &Data<String>,
        offset: isize,
        reverse: bool,
    ) -> Option<WillApply> {
        self.will_apply(patchable, offset, !reverse)
    }

    fn is_already_applied_nearby(
        &self,
        pd: &PatchableData<String, Data<String>>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)> {
        self.will_apply_nearby(pd, next_chunk, offset, !reverse)
    }

    fn already_applied_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut PatchableData<String, Data<String>>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        let after = self.after(reverse);
        let end = after.adj_start(offset, reductions) + after.adj_length(reductions);
        let ok = pd.write_into_upto(into, end)?;
        debug_assert!(ok);
        Ok(())
    }

    fn write_failure_data_into<W: io::Write>(&self, into: &mut W, reverse: bool) -> io::Result<()> {
        into.write_all(b"<<<<<<<\n")?;
        self.before(reverse).write_into(into, None)?;
        into.write_all(b"=======\n")?;
        self.after(reverse).write_into(into, None)?;
        into.write_all(b">>>>>>>\n")
    }
}

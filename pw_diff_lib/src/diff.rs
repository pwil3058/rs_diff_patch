// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::{
    ApplyChunkClean, ApplyChunkFuzzy, ApplyChunksClean, ApplyChunksFuzzy, PatchableData,
    PatchableDataIfce, WillApply,
};
use crate::data::{Data, DataIfce};
use crate::modifications::{ChunkIter, Modifications};
use crate::range::Len;
use crate::snippet::{Snippet, SnippetWrite};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::path::{Path, PathBuf};

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

impl ByteChangeDiff {
    pub fn new(before_file_path: &Path, after_file_path: &Path, context: u8) -> io::Result<Self> {
        let before_bytes = Data::<u8>::read(File::open(before_file_path)?)?;
        let after_bytes = Data::<u8>::read(File::open(after_file_path)?)?;
        let modifications = Modifications::<u8>::new(before_bytes, after_bytes);

        Ok(Self {
            before_path: before_file_path.to_path_buf(),
            after_path: after_file_path.to_path_buf(),
            compressed: false,
            chunks: modifications.chunks::<ByteChangeChunk>(context).collect(),
        })
    }

    pub fn from_reader<R: io::Read>(reader: &mut R) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(reader)
    }

    pub fn before_path(&self) -> &Path {
        &self.before_path
    }

    pub fn after_path(&self) -> &Path {
        &self.after_path
    }

    pub fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }
}

impl<'a> ApplyChunksClean<'a, u8, Data<u8>, ByteChangeChunk> for ByteChangeDiff {
    fn chunks<'b>(&'b self) -> impl Iterator<Item = &'b ByteChangeChunk>
    where
        ByteChangeChunk: 'b,
    {
        self.chunks.iter()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PathAndBytes {
    path: PathBuf,
    compressed: bool,
    bytes: Box<[u8]>,
}

impl crate::diff::PathAndBytes {
    pub fn new(path: &Path) -> io::Result<Self> {
        let mut bytes = vec![];
        let mut reader = BufReader::new(File::open(path)?);
        reader.read_to_end(&mut bytes)?;

        Ok(Self {
            path: path.to_path_buf(),
            compressed: false,
            bytes: bytes.into_boxed_slice(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write_into<W: io::Write>(&self, into: &mut W) -> io::Result<()> {
        into.write_all(&self.bytes)
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TextChangeDiff {
    before_path: PathBuf,
    after_path: PathBuf,
    chunks: Vec<TextChangeChunk>,
}

impl TextChangeDiff {
    pub fn new(before_file_path: &Path, after_file_path: &Path, context: u8) -> io::Result<Self> {
        let before_lines = Data::<String>::read(File::open(before_file_path)?)?;
        let after_lines = Data::<String>::read(File::open(after_file_path)?)?;
        let modifications = Modifications::<String>::new(before_lines, after_lines);

        Ok(Self {
            before_path: before_file_path.to_path_buf(),
            after_path: after_file_path.to_path_buf(),
            chunks: modifications.chunks::<TextChangeChunk>(context).collect(),
        })
    }

    pub fn from_reader<R: io::Read>(reader: &mut R) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(reader)
    }

    pub fn before_path(&self) -> &Path {
        &self.before_path
    }

    pub fn after_path(&self) -> &Path {
        &self.after_path
    }

    pub fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }
}

impl ApplyChunksFuzzy<String, Data<String>, TextChangeChunk> for TextChangeDiff {
    fn chunks<'s>(&'s self) -> impl Iterator<Item = &'s TextChangeChunk>
    where
        TextChangeChunk: 's,
    {
        self.chunks.iter()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PathAndLines {
    path: PathBuf,
    lines: Box<[String]>,
}

impl PathAndLines {
    pub fn new(path: &Path) -> io::Result<Self> {
        let mut lines = vec![];
        let mut reader = BufReader::new(File::open(path)?);
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                break;
            } else {
                lines.push(line)
            }
        }

        Ok(Self {
            path: path.to_path_buf(),
            lines: lines.into_boxed_slice(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn write_into<W: io::Write>(&self, into: &mut W) -> io::Result<()> {
        for line in self.lines.iter() {
            into.write_all(line.as_bytes())?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Diff {
    TextChange(TextChangeDiff),
    TextAdd(PathAndLines),
    TextRemove(PathAndLines),
    ByteChange(ByteChangeDiff),
    ByteAdd(PathAndBytes),
    ByteRemove(PathAndBytes),
}

impl Diff {
    pub fn new(before_file_path: &Path, after_file_path: &Path, context: u8) -> io::Result<Self> {
        if before_file_path.exists() {
            if after_file_path.exists() {
                match TextChangeDiff::new(before_file_path, after_file_path, context) {
                    Ok(text_change_diff) => Ok(Self::TextChange(text_change_diff)),
                    Err(_) => Ok(Self::ByteChange(ByteChangeDiff::new(
                        before_file_path,
                        after_file_path,
                        context,
                    )?)),
                }
            } else {
                match PathAndLines::new(before_file_path) {
                    Ok(path_and_lines) => Ok(Self::TextRemove(path_and_lines)),
                    Err(_) => Ok(Self::ByteRemove(PathAndBytes::new(before_file_path)?)),
                }
            }
        } else if after_file_path.exists() {
            match PathAndLines::new(after_file_path) {
                Ok(mut path_and_lines) => {
                    path_and_lines.path = before_file_path.to_path_buf();
                    Ok(Self::TextAdd(path_and_lines))
                }
                Err(_) => {
                    let mut path_and_bytes = PathAndBytes::new(after_file_path)?;
                    path_and_bytes.path = before_file_path.to_path_buf();
                    Ok(Self::ByteAdd(path_and_bytes))
                }
            }
        } else {
            Err(io::Error::new(
                ErrorKind::NotFound,
                "Neither input file exists!",
            ))
        }
    }

    pub fn from_reader<R: io::Read>(reader: &mut R) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(reader)
    }

    pub fn to_writer<W: io::Write>(&self, writer: &mut W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }
}

#[cfg(test)]
mod diff_tests;

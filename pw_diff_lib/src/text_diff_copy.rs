// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::apply_text_copy::*;
use crate::modifications_copy::*;
use crate::range::Range;
use crate::sequence::*;
use crate::snippet::{Snippet, SnippetWrite};

#[derive(Debug, Serialize, Deserialize)]
pub struct TextChangeChunk {
    context_lengths: (u8, u8),
    before: Snippet<String>,
    after: Snippet<String>,
}

impl From<ModificationChunk<'_, String>> for TextChangeChunk {
    fn from(modn_chunk: ModificationChunk<String>) -> Self {
        let (before_range, after_range) = modn_chunk.ranges();

        TextChangeChunk {
            context_lengths: modn_chunk.context_lengths(),
            before: modn_chunk.before.extract_snippet(before_range),
            after: modn_chunk.after.extract_snippet(after_range),
        }
    }
}

impl ModificationBasics for TextChangeChunk {
    fn before_start(&self, reverse: bool) -> usize {
        if reverse {
            self.after.start
        } else {
            self.before.start
        }
    }

    fn before_end(&self, reverse: bool) -> usize {
        if reverse {
            self.after.start + self.after.items.len()
        } else {
            self.before.start + self.before.items.len()
        }
    }
}

impl TextChunkBasics for TextChangeChunk {
    fn context_lengths(&self) -> (u8, u8) {
        self.context_lengths
    }

    fn before_lines(&self, range: Option<Range>, reverse: bool) -> impl Iterator<Item = &String> {
        if reverse {
            self.after.items(range)
        } else {
            self.before.items(range)
        }
    }
}

impl TextChangeChunk {
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

// impl ApplyChunkFuzzy for TextChangeChunk {}

impl ApplyChunkFuzzy for TextChangeChunk {
    fn will_apply(
        &self,
        patchable: &Seq<String>,
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
        pd: &mut ConsumableSeq<String>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        let before = self.before(reverse);
        let end = before.adj_start(offset, reductions);
        pd.write_into_upto(into, end)?;
        self.after(reverse).write_into(into, reductions)?;
        pd.advance_consumed_by(before.adj_length(reductions));
        Ok(())
    }

    fn will_apply_nearby(
        &self,
        pd: &ConsumableSeq<String>,
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
        patchable: &Seq<String>,
        offset: isize,
        reverse: bool,
    ) -> Option<WillApply> {
        self.will_apply(patchable, offset, !reverse)
    }

    fn is_already_applied_nearby(
        &self,
        pd: &ConsumableSeq<String>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)> {
        self.will_apply_nearby(pd, next_chunk, offset, !reverse)
    }

    fn already_applied_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut ConsumableSeq<String>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        let after = self.after(reverse);
        let end = after.adj_start(offset, reductions) + after.adj_length(reductions);
        pd.write_into_upto(into, end)
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
        let before_lines = Seq::<String>::read(File::open(before_file_path)?)?;
        let after_lines = Seq::<String>::read(File::open(after_file_path)?)?;
        let modifications = Modifications::<String>::new(before_lines, after_lines);

        Ok(Self {
            before_path: before_file_path.to_path_buf(),
            after_path: after_file_path.to_path_buf(),
            chunks: modifications
                .modification_chunks(context)
                .map(|c| TextChangeChunk::from(c))
                .collect(),
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

impl ApplyChunksFuzzy<TextChangeChunk> for TextChangeDiff {
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
        use std::io::BufRead;
        let mut lines = vec![];
        let mut reader = io::BufReader::new(File::open(path)?);
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
    pub fn change_path(&mut self, new_path: &Path) {
        self.path = new_path.to_path_buf()
    }

    pub fn write_into<W: io::Write>(&self, into: &mut W) -> io::Result<()> {
        for line in self.lines.iter() {
            into.write_all(line.as_bytes())?;
        }
        Ok(())
    }
}

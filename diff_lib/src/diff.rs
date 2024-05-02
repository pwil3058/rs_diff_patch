// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::{Applies, ApplyChunk, ApplyChunks, ProgressData};
use crate::lines::{DiffableLines, PatchableLines};
use crate::modifications::ChunkIter;
use crate::range::Range;
use crate::snippet::Snippet;
use std::fs::File;
use std::io;

use crate::{Lines, Modifications};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeChunk {
    context_lengths: (usize, usize),
    before: Snippet,
    after: Snippet,
}

impl<'a, A: DiffableLines, P: DiffableLines> Iterator for ChunkIter<'a, A, P, ChangeChunk> {
    type Item = ChangeChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let modn_chunk = self.iter.next()?;
        let (before_range, after_range) = modn_chunk.ranges();

        Some(ChangeChunk {
            context_lengths: modn_chunk.context_lengths(),
            before: self.before.extract_snippet(before_range),
            after: self.after.extract_snippet(after_range),
        })
    }
}

impl ChangeChunk {
    pub fn before(&self, reverse: bool) -> &Snippet {
        if reverse {
            &self.after
        } else {
            &self.before
        }
    }

    pub fn after(&self, reverse: bool) -> &Snippet {
        if reverse {
            &self.before
        } else {
            &self.after
        }
    }
}

impl ApplyChunk for ChangeChunk {
    fn before_lines_as_text(&self, reductions: Option<(usize, usize)>, reverse: bool) -> String {
        if reverse {
            self.after.lines_as_text(reductions)
        } else {
            self.before.lines_as_text(reductions)
        }
    }

    fn after_lines_as_text(&self, reductions: Option<(usize, usize)>, reverse: bool) -> String {
        if reverse {
            self.before.lines_as_text(reductions)
        } else {
            self.after.lines_as_text(reductions)
        }
    }

    fn applies(
        &self,
        lines: &impl PatchableLines,
        offset: isize,
        reverse: bool,
    ) -> Option<Applies> {
        let before = self.before(reverse);
        let start = before.start as isize + offset;
        if !start.is_negative() && lines.matches_at(&before.lines, start as usize) {
            Some(Applies::Cleanly)
        } else {
            let max_reduction = self.context_lengths.0.max(self.context_lengths.1);
            for redn in 1..max_reduction {
                let start_redn = redn.min(self.context_lengths.0);
                let end_redn = redn.min(self.context_lengths.1);
                let adj_start = start + start_redn as isize;
                if !adj_start.is_negative()
                    && lines.matches_at(
                        &before.lines[start_redn..before.length(None) - end_redn],
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
        lines: &impl PatchableLines,
        not_before: usize,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, Applies)> {
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
                - before.length(Some(self.context_lengths))
        } else {
            lines.len() - before.length(Some(self.context_lengths))
        };
        let mut backward_done = false;
        let mut forward_done = false;
        for i in 1isize.. {
            if !backward_done {
                let adjusted_offset = offset - i;
                if before.start as isize + adjusted_offset < not_before as isize {
                    backward_done = true;
                } else {
                    if let Some(applies) = self.applies(lines, adjusted_offset, reverse) {
                        return Some((-i, applies));
                    }
                }
            }
            if !forward_done {
                let adjusted_offset = offset + i;
                if before.start as isize + adjusted_offset < not_after as isize {
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
        L: PatchableLines,
        W: Write,
    {
        let before = self.before(reverse);
        let end = before.start(pd.offset, reductions);
        let text = pd.lines.lines_as_text(Range(pd.consumed, end));
        into.write_all(text.as_bytes())?;
        into.write_all(self.after_lines_as_text(reductions, reverse).as_bytes())?;
        pd.consumed = end + before.length(reductions);
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
        L: PatchableLines,
        W: Write,
    {
        let after = self.after(reverse);
        let end = after.start(pd.offset, reductions) + after.length(reductions);
        let text = pd.lines.lines_as_text(Range(pd.consumed, end));
        into.write_all(text.as_bytes())?;
        pd.consumed = end;
        Ok(())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ChangeDiff {
    before_path: PathBuf,
    after_path: PathBuf,
    chunks: Vec<ChangeChunk>,
}

impl ChangeDiff {
    pub fn new(
        before_file_path: &Path,
        after_file_path: &Path,
        context: usize,
    ) -> io::Result<Self> {
        let before_lines = Lines::read(File::open(before_file_path)?)?;
        let after_lines = Lines::read(File::open(after_file_path)?)?;
        let modifications = Modifications::new(before_lines, after_lines);

        Ok(Self {
            before_path: before_file_path.to_path_buf(),
            after_path: after_file_path.to_path_buf(),
            chunks: modifications.chunks::<ChangeChunk>(context).collect(),
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

impl<'a> ApplyChunks<'a, ChangeChunk> for ChangeDiff {
    fn chunks<'s>(&'s self) -> impl Iterator<Item = &'s ChangeChunk>
    where
        ChangeChunk: 's,
    {
        self.chunks.iter()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PathAndContent {
    path: PathBuf,
    lines: Vec<String>,
}

impl PathAndContent {
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
            lines,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Diff {
    Change(ChangeDiff),
    Create(PathAndContent),
    Delete(PathAndContent),
}

impl Diff {
    pub fn new(
        before_file_path: &Path,
        after_file_path: &Path,
        context: usize,
    ) -> io::Result<Self> {
        if before_file_path.exists() {
            if after_file_path.exists() {
                Ok(Self::Change(ChangeDiff::new(
                    before_file_path,
                    after_file_path,
                    context,
                )?))
            } else {
                Ok(Self::Delete(PathAndContent::new(before_file_path)?))
            }
        } else if after_file_path.exists() {
            let mut pac = PathAndContent::new(after_file_path)?;
            pac.path = before_file_path.to_path_buf();
            Ok(Self::Create(pac))
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
mod tests;

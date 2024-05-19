// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::apply_text_copy::*;
use crate::modifications_copy::*;
use crate::range::Range;
use crate::sequence::*;
use crate::snippet::Snippet;

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

impl ApplyChunkFuzzy for TextChangeChunk {}

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

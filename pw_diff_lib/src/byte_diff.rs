// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::{ApplyChunkClean, ApplyChunksClean};
use crate::data::{ConsumableData, ConsumableDataIfce, Data, DataIfce};
use crate::modifications::{ChunkIter, Modifications};
use crate::range::Len;
use crate::snippet::{Snippet, SnippetWrite};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io;
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
        pd: &mut ConsumableData<u8, Data<u8>>,
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
        pd: &mut ConsumableData<u8, Data<u8>>,
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

impl PathAndBytes {
    pub fn new(path: &Path) -> io::Result<Self> {
        use std::io::Read;
        let mut bytes = vec![];
        let mut reader = io::BufReader::new(File::open(path)?);
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

    pub fn change_path(&mut self, new_path: &Path) {
        self.path = new_path.to_path_buf()
    }

    pub fn write_into<W: io::Write>(&self, into: &mut W) -> io::Result<()> {
        into.write_all(&self.bytes)
    }
}

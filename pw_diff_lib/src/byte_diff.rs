// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::apply_bytes::{ApplyClumpClean, ApplyClumpsClean};
use crate::changes::{ChangeClump, Changes};
use crate::range::Len;
use crate::snippet::{Snippet, SnippetWrite};

use crate::sequence::{ConsumableSeq, ConsumableSeqIfce, Seq};

#[derive(Debug, Serialize, Deserialize)]
pub struct ByteChangeClump {
    context_lengths: (u8, u8),
    before: Snippet<u8>,
    after: Snippet<u8>,
}

impl From<ChangeClump<'_, u8>> for ByteChangeClump {
    fn from(change_clump: ChangeClump<'_, u8>) -> Self {
        let (before_range, after_range) = change_clump.ranges();

        ByteChangeClump {
            context_lengths: change_clump.context_lengths(),
            before: change_clump.before.extract_snippet(before_range),
            after: change_clump.after.extract_snippet(after_range),
        }
    }
}

impl ByteChangeClump {
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

impl<'a> ApplyClumpClean for ByteChangeClump {
    fn will_apply(&self, data: &Seq<u8>, reverse: bool) -> bool {
        let before = self.before(reverse);
        data.has_subsequence_at(&before.items, before.start)
    }

    fn is_already_applied(&self, data: &Seq<u8>, reverse: bool) -> bool {
        let after = self.after(reverse);
        data.has_subsequence_at(&after.items, after.start)
    }

    fn apply_into<W: io::Write>(
        &self,
        pd: &mut ConsumableSeq<u8>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<()> {
        let before = self.before(reverse);
        pd.write_into_upto(into, before.start)?;
        self.after(reverse).write_into(into, None)?;
        pd.advance_consumed_by(before.len());
        Ok(())
    }

    fn already_applied_into<W: io::Write>(
        &self,
        pd: &mut ConsumableSeq<u8>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<()> {
        let after = self.after(reverse);
        pd.write_into_upto(into, after.start + after.len())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ByteChangeDiff {
    before_path: PathBuf,
    after_path: PathBuf,
    compressed: bool,
    clumps: Box<[ByteChangeClump]>,
}

impl ByteChangeDiff {
    pub fn new(before_file_path: &Path, after_file_path: &Path, context: u8) -> io::Result<Self> {
        let before_bytes = Seq::<u8>::read(File::open(before_file_path)?)?;
        let after_bytes = Seq::<u8>::read(File::open(after_file_path)?)?;
        let modifications = Changes::<u8>::new(before_bytes, after_bytes);

        Ok(Self {
            before_path: before_file_path.to_path_buf(),
            after_path: after_file_path.to_path_buf(),
            compressed: false,
            clumps: modifications
                .change_clumps(context)
                .map(|c| ByteChangeClump::from(c))
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

impl ApplyClumpsClean<'_, ByteChangeClump> for ByteChangeDiff {
    fn clumps<'b>(&'b self) -> impl Iterator<Item = &'b ByteChangeClump>
    where
        ByteChangeClump: 'b,
    {
        self.clumps.iter()
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

// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use longest_common_subsequence::changes::Changes;
use longest_common_subsequence::sequence::SequenceIO;
use longest_common_subsequence::{range::*, sequence::Seq};
use pw_diff_lib::apply_bytes::{ApplyClumpClean, ApplyClumpsClean};
use pw_diff_lib::sequence::{ConsumableSeq, ConsumableSeqIfce};
use pw_diff_lib::snippet::*;
use serde::{Deserialize, Serialize};
use serde_layers::human_binary;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct BinarySnippet {
    pub start: usize,
    #[serde(with = "human_binary")]
    pub items: Box<[u8]>,
}

impl Len for BinarySnippet {
    #[inline]
    fn len(&self) -> usize {
        self.items.len()
    }
}

impl SnippetIfce for BinarySnippet {
    type Item = u8;
    #[inline]
    fn start(&self) -> usize {
        self.start
    }
    #[inline]
    fn items_slice(&self) -> &[Self::Item] {
        &self.items
    }
}

pub trait ExtractSnippet<S: SnippetIfce> {
    fn extract_snippet(&self, range: Range) -> S;
}

impl ExtractSnippet<BinarySnippet> for Seq<u8> {
    fn extract_snippet(&self, range: Range) -> BinarySnippet {
        let start = range.start().min(self.len());
        let end = range.end().min(self.len());
        BinarySnippet {
            start,
            items: self.0[start..end].to_vec().into_boxed_slice(),
        }
    }
}

pub trait SnippetWrite {
    fn write_into<W: Write>(&self, writer: &mut W, reductions: Option<(u8, u8)>) -> io::Result<()>;
}

impl SnippetWrite for BinarySnippet {
    fn write_into<W: Write>(&self, writer: &mut W, reductions: Option<(u8, u8)>) -> io::Result<()> {
        let range = self.range(reductions);
        for &byte in self.items(Some(range)) {
            writer.write_all(&[byte])?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BinaryChangeClump {
    pub context_lengths: (u8, u8),
    pub before: BinarySnippet,
    pub after: BinarySnippet,
}

impl From<longest_common_subsequence::changes::ChangeClump<'_, u8>> for BinaryChangeClump {
    fn from(change_clump: longest_common_subsequence::changes::ChangeClump<u8>) -> Self {
        let (before_range, after_range) = change_clump.ranges();

        BinaryChangeClump {
            context_lengths: change_clump.context_lengths(),
            before: change_clump.before.extract_snippet(before_range),
            after: change_clump.after.extract_snippet(after_range),
        }
    }
}

impl ApplyClumpClean for BinaryChangeClump {
    fn will_apply(&self, patchable: &Seq<u8>, reverse: bool) -> bool {
        // Since fuzzy matching is skipped, offset is implicitly 0 and reductions are None
        let target_snippet = if reverse { &self.after } else { &self.before };
        let start = target_snippet.start;
        let len = target_snippet.len();
        let end = start + len;

        if end > patchable.len() {
            false
        } else {
            // High-speed slice comparison using the exact byte bounds
            let target_range = Range(start, end);
            target_snippet.items_slice() == &patchable[target_range.0..target_range.1]
        }
    }

    fn is_already_applied(&self, patchable: &Seq<u8>, reverse: bool) -> bool {
        // To check if already applied, we invert the target check direction (!reverse)
        self.will_apply(patchable, !reverse)
    }

    fn apply_into<W: io::Write>(
        &self,
        pd: &mut ConsumableSeq<u8>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<()> {
        let source_snippet = if reverse { &self.after } else { &self.before };
        let destination_snippet = if reverse { &self.before } else { &self.after };

        // 1. Fast-forward the consumable sequence up to our clean byte start position
        pd.write_into_upto(into, source_snippet.start)?;

        // 2. Write the replacement binary data payload into the file stream
        destination_snippet.write_into(into, None)?;

        // 3. Advance the consumed index boundary by the exact size of the removed payload
        pd.advance_consumed_by(source_snippet.len());

        Ok(())
    }

    fn already_applied_into<W: io::Write>(
        &self,
        pd: &mut ConsumableSeq<u8>,
        into: &mut W,
        _reverse: bool,
    ) -> io::Result<()> {
        // If it's already applied, we just fast-forward the consumable cursor past
        // the modified block without writing new replacement data bytes.
        let target_snippet = if _reverse { &self.before } else { &self.after };
        let end_pos = target_snippet.start + target_snippet.len();
        pd.write_into_upto(into, end_pos)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BinaryChangeDiff {
    pub before_path: PathBuf,
    pub after_path: PathBuf,
    clumps: Vec<BinaryChangeClump>,
}

impl BinaryChangeDiff {
    pub fn new(
        before_file_path: &std::path::Path,
        after_file_path: &std::path::Path,
        context: u8,
    ) -> io::Result<Self> {
        // High-speed file reader ingest into Seq<u8> buffers using BufReader
        let before_bytes =
            Seq::<u8>::read_from(io::BufReader::new(std::fs::File::open(before_file_path)?))?;
        let after_bytes =
            Seq::<u8>::read_from(io::BufReader::new(std::fs::File::open(after_file_path)?))?;

        let changes = Changes::<u8>::new(&before_bytes, &after_bytes);

        Ok(Self {
            before_path: before_file_path.to_path_buf(),
            after_path: after_file_path.to_path_buf(),
            clumps: changes
                .change_clumps(context)
                .map(BinaryChangeClump::from)
                .collect(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.clumps.is_empty()
    }

    pub fn before_path(&self) -> &std::path::Path {
        &self.before_path
    }

    pub fn after_path(&self) -> &std::path::Path {
        &self.after_path
    }

    pub fn to_writer<W: io::Write>(
        &self,
        writer: W,
        pretty: bool,
    ) -> Result<(), serde_json::Error> {
        let buffered = io::BufWriter::new(writer);
        if pretty {
            serde_json::to_writer_pretty(buffered, self)
        } else {
            serde_json::to_writer(buffered, self)
        }
    }

    pub fn from_reader<R: io::Read>(reader: R) -> Result<Self, serde_json::Error> {
        let buffered = io::BufReader::new(reader);
        serde_json::from_reader(buffered)
    }
}

impl<'a> ApplyClumpsClean<'a, BinaryChangeClump> for BinaryChangeDiff {
    fn clumps<'b>(&'b self) -> impl Iterator<Item = &'b BinaryChangeClump>
    where
        BinaryChangeClump: 'b,
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

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
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

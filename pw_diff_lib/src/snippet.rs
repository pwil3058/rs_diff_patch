// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::io;
use std::io::Write;

use serde::{Deserialize, Serialize};

use longest_common_subsequence::range::{Len, Range};
use longest_common_subsequence::sequence::Seq;
use serde_layers::{human_binary, human_lines};

/// Unified interface providing common properties across both text and binary snippets
pub trait SnippetIfce: Len {
    type Item;

    fn start(&self) -> usize;
    fn items_slice(&self) -> &[Self::Item];

    #[inline]
    fn range(&self, reductions: Option<(u8, u8)>) -> Range {
        if let Some((start_reduction, end_reduction)) = reductions {
            Range(
                start_reduction as usize,
                self.len().saturating_sub(end_reduction as usize),
            )
        } else {
            Range(0, self.len())
        }
    }

    #[inline]
    fn adj_length(&self, reductions: Option<(u8, u8)>) -> usize {
        if let Some((start_reduction, end_reduction)) = reductions {
            self.len()
                .saturating_sub(start_reduction as usize + end_reduction as usize)
        } else {
            self.len()
        }
    }

    #[inline]
    fn adj_start(&self, offset: isize, reductions: Option<(u8, u8)>) -> usize {
        let base_start = self.start().checked_add_signed(offset).expect("underflow");
        if let Some(reductions) = reductions {
            base_start + reductions.0 as usize
        } else {
            base_start
        }
    }

    #[inline]
    fn items(&self, range: Option<Range>) -> impl Iterator<Item = &Self::Item> {
        if let Some(range) = range {
            debug_assert!(range.is_valid_for_max_end(self.len() + self.start()));
            self.items_slice()[range.0..range.1].iter()
        } else {
            self.items_slice().iter()
        }
    }
}

// =========================================================================
// 1. Text Snippet Realization
// =========================================================================

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct TextSnippet {
    pub start: usize,
    #[serde(with = "human_lines")]
    pub items: Box<[String]>,
}

impl Len for TextSnippet {
    #[inline]
    fn len(&self) -> usize {
        self.items.len()
    }
}

impl SnippetIfce for TextSnippet {
    type Item = String;
    #[inline]
    fn start(&self) -> usize {
        self.start
    }
    #[inline]
    fn items_slice(&self) -> &[Self::Item] {
        &self.items
    }
}

// =========================================================================
// 2. Binary Snippet Realization
// =========================================================================

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

// =========================================================================
// 3. Extraction Traits and I/O Write Implementations
// =========================================================================

pub trait ExtractSnippet<S: SnippetIfce> {
    fn extract_snippet(&self, range: Range) -> S;
}

impl ExtractSnippet<TextSnippet> for Seq<String> {
    fn extract_snippet(&self, range: Range) -> TextSnippet {
        TextSnippet {
            start: range.start(),
            items: self.0[range.0..range.1].to_vec().into_boxed_slice(),
        }
    }
}

impl ExtractSnippet<BinarySnippet> for Seq<u8> {
    fn extract_snippet(&self, range: Range) -> BinarySnippet {
        BinarySnippet {
            start: range.start(),
            items: self.0[range.0..range.1].to_vec().into_boxed_slice(),
        }
    }
}

pub trait SnippetWrite {
    fn write_into<W: Write>(&self, writer: &mut W, reductions: Option<(u8, u8)>) -> io::Result<()>;
}

impl SnippetWrite for TextSnippet {
    fn write_into<W: Write>(&self, writer: &mut W, reductions: Option<(u8, u8)>) -> io::Result<()> {
        let range = self.range(reductions);
        for string in self.items(Some(range)) {
            writer.write_all(string.as_bytes())?;
        }
        Ok(())
    }
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

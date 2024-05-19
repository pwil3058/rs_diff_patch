// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::range::{Len, Range};
use serde::{Deserialize, Serialize};
use std::io;
use std::io::Write;

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snippet<T> {
    pub start: usize,
    pub items: Box<[T]>,
}

impl<T> Len for Snippet<T> {
    fn len(&self) -> usize {
        self.items.len()
    }
}

impl<T> Snippet<T> {
    pub fn range(&self, reductions: Option<(u8, u8)>) -> Range {
        if let Some((start_reduction, end_reduction)) = reductions {
            Range(
                start_reduction as usize,
                self.items.len() - end_reduction as usize,
            )
        } else {
            Range(0, self.items.len())
        }
    }

    pub fn adj_length(&self, reductions: Option<(u8, u8)>) -> usize {
        if let Some((start_reduction, end_reduction)) = reductions {
            self.items.len() - start_reduction as usize - end_reduction as usize
        } else {
            self.items.len()
        }
    }

    pub fn adj_start(&self, offset: isize, reductions: Option<(u8, u8)>) -> usize {
        if let Some(reductions) = reductions {
            reductions.0 as usize + self.start.checked_add_signed(offset).expect("underflow")
        } else {
            self.start.checked_add_signed(offset).expect("underflow")
        }
    }

    pub fn items(&self, range: Option<Range>) -> impl Iterator<Item = &T> {
        if let Some(range) = range {
            debug_assert!(range.is_valid_for_max_end(self.len() + self.start));
            self.items[range.0..range.1].iter()
        } else {
            self.items.iter()
        }
    }
}

pub trait SnippetWrite {
    fn write_into<W: Write>(&self, writer: &mut W, reductions: Option<(u8, u8)>) -> io::Result<()>;
}

impl SnippetWrite for Snippet<u8> {
    fn write_into<W: Write>(&self, writer: &mut W, reductions: Option<(u8, u8)>) -> io::Result<()> {
        if let Some((start, end)) = reductions {
            writer.write_all(&self.items[start as usize..self.items.len() - end as usize])
        } else {
            writer.write_all(&self.items)
        }
    }
}
impl SnippetWrite for Snippet<String> {
    fn write_into<W: Write>(&self, writer: &mut W, reductions: Option<(u8, u8)>) -> io::Result<()> {
        if let Some((start, end)) = reductions {
            for string in self.items[start as usize..self.items.len() - end as usize].iter() {
                writer.write_all(string.as_bytes())?;
            }
        } else {
            for string in self.items.iter() {
                writer.write_all(string.as_bytes())?;
            }
        }
        Ok(())
    }
}

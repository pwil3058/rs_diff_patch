// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use longest_common_subsequence::snippet::*;

use std::io;
use std::io::Write;

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

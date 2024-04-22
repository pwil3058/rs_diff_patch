// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::ApplyInto;
use crate::diff::DiffChunk;
use crate::lines::LazyLines;
use crate::modifications::Modifications;
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Serialize, Deserialize)]
struct WrappedDiffChunks(pub Vec<DiffChunk>);

impl<'a> ApplyInto<'a, DiffChunk> for WrappedDiffChunks {
    fn chunks<'s>(&'s self) -> impl Iterator<Item = &'s DiffChunk>
    where
        DiffChunk: 's,
    {
        self.0.iter()
    }
}

#[derive(Default)]
struct WrappedString(pub String);

impl Write for WrappedString {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            self.0.push_str(&std::str::from_utf8_unchecked(buf));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn clean_patch() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    let after_lines = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    let modifications =
        Modifications::new(LazyLines::from(before_lines), LazyLines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = modifications.chunks::<DiffChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WrappedString::default();
    patch
        .apply_into(&LazyLines::from(before_lines), &mut patched, false)
        .unwrap();
    assert_eq!(patched.0, after_lines);
}

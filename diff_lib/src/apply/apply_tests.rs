// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::ApplyInto;
use crate::diff::DiffChunk;
use crate::lines::Lines;
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
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = modifications.chunks::<DiffChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WrappedString::default();
    patch
        .apply_into(&Lines::from(before_lines), &mut patched, false)
        .unwrap();
    assert_eq!(patched.0, after_lines);
}

#[test]
fn clean_patch_in_middle() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = modifications.chunks::<DiffChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WrappedString::default();
    patch
        .apply_into(&Lines::from(before_lines), &mut patched, false)
        .unwrap();
    assert_eq!(patched.0, after_lines);
}

#[test]
fn already_applied() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = modifications.chunks::<DiffChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WrappedString::default();
    patch
        .apply_into(&Lines::from(after_lines), &mut patched, false)
        .unwrap();
    assert_eq!(patched.0, after_lines);
}

#[test]
fn clean_patch_reverse() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = modifications.chunks::<DiffChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WrappedString::default();
    patch
        .apply_into(&Lines::from(after_lines), &mut patched, true)
        .unwrap();
    assert_eq!(patched.0, before_lines);
}

#[test]
fn displaced() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = modifications.chunks::<DiffChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WrappedString::default();
    patch
        .apply_into(
            &Lines::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(patched.0, "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn displaced_no_final_eol_1() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = modifications.chunks::<DiffChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WrappedString::default();
    patch
        .apply_into(
            &Lines::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(patched.0, "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn displaced_no_final_eol_2() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\na";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = modifications.chunks::<DiffChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WrappedString::default();
    patch
        .apply_into(
            &Lines::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(patched.0, "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn displaced_no_final_eol_3() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = modifications.chunks::<DiffChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WrappedString::default();
    patch
        .apply_into(
            &Lines::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(patched.0, "x\ny\nz\n".to_owned() + after_lines);
}

// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::ApplyChunks;
use crate::diff::ChangeChunk;
use crate::lines::Lines;
use crate::modifications::Modifications;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Write};
use std::ops::{Deref, DerefMut};

#[derive(Serialize, Deserialize)]
struct WrappedDiffChunks(pub Vec<ChangeChunk>);

impl<'a> ApplyChunks<'a, ChangeChunk> for WrappedDiffChunks {
    fn chunks<'s>(&'s self) -> impl Iterator<Item = &'s ChangeChunk>
    where
        ChangeChunk: 's,
    {
        self.0.iter()
    }
}

#[derive(Debug, Default)]
struct WriteableString(Cursor<Vec<u8>>);

impl Deref for WriteableString {
    type Target = Cursor<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WriteableString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Write for WriteableString {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl PartialEq<String> for WriteableString {
    fn eq(&self, other: &String) -> bool {
        &String::from_utf8(self.get_ref().clone()).unwrap() == other
    }
}

#[test]
fn clean_patch() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    let after_lines = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WriteableString::default();

    let stats = patch
        .apply_into(&Lines::from(before_lines), &mut patched, false)
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched, after_lines.to_string());
}

#[test]
fn clean_patch_in_middle() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WriteableString::default();
    let stats = patch
        .apply_into(&Lines::from(before_lines), &mut patched, false)
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched, after_lines.to_string());
}

#[test]
fn already_partially_applied() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WriteableString::default();
    let stats = patch
        .apply_into(&Lines::from(after_lines), &mut patched, false)
        .unwrap();
    assert_eq!(stats.clean, 0);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 2);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched, after_lines.to_string());
}

#[test]
fn clean_patch_reverse() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WriteableString::default();
    let stats = patch
        .apply_into(&Lines::from(after_lines), &mut patched, true)
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched, before_lines.to_string());
}

#[test]
fn displaced() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WriteableString::default();
    let stats = patch
        .apply_into(
            &Lines::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 1);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched, "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn displaced_no_final_eol_1() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WriteableString::default();
    let stats = patch
        .apply_into(
            &Lines::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched, "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn displaced_no_final_eol_2() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\na";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WriteableString::default();
    let stats = patch
        .apply_into(
            &Lines::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched, "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn displaced_no_final_eol_3() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    let mut patched = WriteableString::default();
    let stats = patch
        .apply_into(
            &Lines::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched, "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn already_applied() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    let patch = WrappedDiffChunks(diff_chunks);
    assert!(patch.already_applied(&Lines::from(after_lines), false));
    assert!(!patch.already_applied(&Lines::from(before_lines), false));
    assert!(patch.already_applied(&Lines::from("x\ny\nz\n".to_owned() + after_lines), false));
}

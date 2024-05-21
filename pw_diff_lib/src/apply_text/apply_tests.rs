// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::io::BufWriter;

use serde::{Deserialize, Serialize};

use crate::apply_text::*;
use crate::changes::*;
use crate::sequence::*;
use crate::text_diff::*;

#[derive(Serialize, Deserialize)]
struct WrappedDiffClumps(pub Vec<TextChangeClump>);

impl ApplyClumpsFuzzy<TextChangeClump> for WrappedDiffClumps {
    fn clumps<'s>(&'s self) -> impl Iterator<Item = &'s TextChangeClump>
    where
        TextChangeClump: 's,
    {
        self.0.iter()
    }
}

trait Stringy {
    fn to_string(&self) -> String;
}

impl Stringy for BufWriter<Vec<u8>> {
    fn to_string(&self) -> String {
        String::from_utf8(self.buffer().to_vec()).unwrap()
    }
}

#[test]
fn clean_patch() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    let after_lines = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    let modifications =
        Changes::<String>::new(Seq::from(before_lines), Seq::from(after_lines));
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);

    let stats = patch
        .apply_into(&Seq::from(before_lines), &mut patched, false)
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched.to_string(), after_lines.to_string());
}

#[test]
fn clean_patch_in_middle() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications =
        Changes::<String>::new(Seq::from(before_lines), Seq::from(after_lines));
    let diff_lumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_lumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(&Seq::from(before_lines), &mut patched, false)
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched.to_string(), after_lines.to_string());
}

#[test]
fn already_fully_applied() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications =
        Changes::<String>::new(Seq::from(before_lines), Seq::from(after_lines));
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(&Seq::from(after_lines), &mut patched, false)
        .unwrap();
    assert_eq!(stats.clean, 0);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 2);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched.to_string(), after_lines.to_string());
}

#[test]
fn clean_patch_reverse() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications =
        Changes::<String>::new(Seq::from(before_lines), Seq::from(after_lines));
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(&Seq::from(after_lines), &mut patched, true)
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched.to_string(), before_lines.to_string());
}

#[test]
fn displaced() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications =
        Changes::<String>::new(Seq::from(before_lines), Seq::from(after_lines));
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(
            &Seq::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 1);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched.to_string(), "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn displaced_no_final_eol_1() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let modifications =
        Changes::<String>::new(Seq::from(before_lines), Seq::from(after_lines));
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(
            &Seq::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched.to_string(), "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn displaced_no_final_eol_2() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\na";
    let modifications =
        Changes::<String>::new(Seq::from(before_lines), Seq::from(after_lines));
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(
            &Seq::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched.to_string(), "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn displaced_no_final_eol_3() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz";
    let modifications =
        Changes::<String>::new(Seq::from(before_lines), Seq::from(after_lines));
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(
            &Seq::from("x\ny\nz\n".to_owned() + before_lines),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched.to_string(), "x\ny\nz\n".to_owned() + after_lines);
}

#[test]
fn already_applied() {
    let before_lines = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz";
    let modifications =
        Changes::<String>::new(Seq::from(before_lines), Seq::from(after_lines));
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    assert!(patch.is_already_applied(&Seq::from(after_lines), false));
    assert!(!patch.is_already_applied(&Seq::from(before_lines), false));
    assert!(patch.is_already_applied(&Seq::from("x\ny\nz\n".to_owned() + after_lines), false));
}

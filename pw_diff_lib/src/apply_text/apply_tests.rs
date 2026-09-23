// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::io::BufWriter;

use serde::{Deserialize, Serialize};

use crate::apply_text::*;
use crate::changes::*;
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

fn line_seq(text: &str) -> Seq<String> {
    Seq::from_iter(text.split_inclusive('\n').map(|s| s.to_string()))
}

#[test]
fn clean_patch() {
    let before_lines_str = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    let after_lines_str = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    let before_lines = line_seq(before_lines_str);
    let after_lines = line_seq(after_lines_str);
    let modifications = Changes::<String>::new(&before_lines, &after_lines);
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);

    let stats = patch
        .apply_into(&line_seq(before_lines_str), &mut patched, false)
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(patched.to_string(), after_lines_str.to_string());
}

#[test]
fn clean_patch_in_middle() {
    let before_lines_str = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines_str = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let before_lines = line_seq(before_lines_str);
    let after_lines = line_seq(after_lines_str);
    let modifications = Changes::<String>::new(&before_lines, &after_lines);
    let diff_lumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_lumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(&before_lines, &mut patched, false)
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(line_seq(&patched.to_string()), after_lines);
}

#[test]
fn already_fully_applied() {
    let before_lines_str = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines_str = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let before_lines = line_seq(before_lines_str);
    let after_lines = line_seq(after_lines_str);
    let modifications = Changes::<String>::new(&before_lines, &after_lines);
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch.apply_into(&after_lines, &mut patched, false).unwrap();
    assert_eq!(stats.clean, 0);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 2);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(line_seq(&patched.to_string()), after_lines);
}

#[test]
fn clean_patch_reverse() {
    let before_lines = line_seq("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n");
    let after_lines = line_seq("a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n");
    let modifications = Changes::<String>::new(&before_lines, &after_lines);
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch.apply_into(&after_lines, &mut patched, true).unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 0);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(line_seq(&patched.to_string()), before_lines);
}

#[test]
fn displaced() {
    let before_lines_str = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let before_lines = line_seq(before_lines_str);
    let after_lines_str = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let after_lines = line_seq(after_lines_str);
    let modifications = Changes::<String>::new(&before_lines, &after_lines);
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(
            &line_seq(&("x\ny\nz\n".to_owned() + before_lines_str)),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 1);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(
        patched.to_string(),
        "x\ny\nz\n".to_owned() + after_lines_str
    );
}

#[test]
fn displaced_no_final_eol_1() {
    let before_lines_str = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz";
    let after_lines_str = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\n";
    let before_lines = line_seq(before_lines_str);
    let after_lines = line_seq(after_lines_str);
    let modifications = Changes::<String>::new(&before_lines, &after_lines);
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(
            &line_seq(&("x\ny\nz\n".to_owned() + before_lines_str)),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(
        patched.to_string(),
        "x\ny\nz\n".to_owned() + after_lines_str
    );
}

#[test]
fn displaced_no_final_eol_2() {
    let before_lines_str = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines_str = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz\na";
    let before_lines = line_seq(before_lines_str);
    let after_lines = line_seq(after_lines_str);
    let modifications = Changes::<String>::new(&before_lines, &after_lines);
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(
            &line_seq(&("x\ny\nz\n".to_owned() + before_lines_str)),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(
        patched.to_string(),
        "x\ny\nz\n".to_owned() + after_lines_str
    );
}

#[test]
fn displaced_no_final_eol_3() {
    let before_lines_str = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines_str = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz";
    let before_lines = line_seq(before_lines_str);
    let after_lines = line_seq(after_lines_str);
    let modifications = Changes::<String>::new(&before_lines, &after_lines);
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    let mut patched = BufWriter::new(vec![]);
    let stats = patch
        .apply_into(
            &line_seq(&("x\ny\nz\n".to_owned() + before_lines_str)),
            &mut patched,
            false,
        )
        .unwrap();
    assert_eq!(stats.clean, 2);
    assert_eq!(stats.fuzzy, 1);
    assert_eq!(stats.already_applied, 0);
    assert_eq!(stats.already_applied_fuzzy, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(
        patched.to_string(),
        "x\ny\nz\n".to_owned() + after_lines_str
    );
}

#[test]
fn already_applied() {
    let before_lines_str = "a\nb\nc\nd\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nx\ny\nz\n";
    let after_lines_str = "a\nb\nc\nd\nA\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\nx\ny\nz";
    let before_lines = line_seq(before_lines_str);
    let after_lines = line_seq(after_lines_str);
    let modifications = Changes::<String>::new(&before_lines, &after_lines);
    let diff_clumps: Vec<TextChangeClump> = modifications
        .change_clumps(2)
        .map(|c| TextChangeClump::from(c))
        .collect();
    let patch = WrappedDiffClumps(diff_clumps);
    assert!(patch.is_already_applied(&after_lines, false));
    assert!(!patch.is_already_applied(&before_lines, false));
    assert!(patch.is_already_applied(
        &line_seq(&("x\ny\nz\n".to_owned() + after_lines_str)),
        false
    ));
}

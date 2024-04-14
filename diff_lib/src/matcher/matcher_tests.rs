// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>
use crate::lines::*;
use crate::matcher::*;

#[test]
fn diff_chunk_applies() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    let after_lines = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    let matcher = Matcher::new(LazyLines::from(before_lines), LazyLines::from(after_lines));
    let diff_chunks: Vec<DiffChunk> = matcher.diff_chunks(2).collect();

    for diff_chunk in diff_chunks.iter() {
        assert_eq!(
            diff_chunk.applies(&LazyLines::from(before_lines)),
            Some(Applies::Cleanly)
        );
        assert_eq!(diff_chunk.applies(&LazyLines::from(after_lines)), None);
    }
    let diff_chunk = diff_chunks.first().unwrap();
    assert_eq!(
        diff_chunk.applies(&LazyLines::from("B\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n")),
        Some(Applies::WithReductions((1, 1)))
    );
}

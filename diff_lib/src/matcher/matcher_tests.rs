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
            diff_chunk.applies(&LazyLines::from(before_lines), 0, false),
            Some(Applies::Cleanly)
        );
        assert_eq!(
            diff_chunk.applies(&LazyLines::from(before_lines), 0, true),
            None
        );
        assert_eq!(
            diff_chunk.applies(&LazyLines::from(after_lines), 0, false),
            None
        );
        assert_eq!(
            diff_chunk.applies(&LazyLines::from(after_lines), 0, true),
            Some(Applies::Cleanly)
        );
    }

    for (i, diff_chunk) in diff_chunks.iter().enumerate() {
        assert_eq!(
            diff_chunk.applies(
                &LazyLines::from("a\na\na\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n"),
                3,
                false
            ),
            Some(Applies::Cleanly)
        );
        assert_eq!(
            diff_chunk.applies(
                &LazyLines::from("B\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n"),
                -1,
                false
            ),
            if i > 0 {
                Some(Applies::Cleanly)
            } else {
                Some(Applies::WithReductions((1, 1)))
            }
        );
    }

    let diff_chunk = diff_chunks.first().unwrap();
    assert_eq!(
        diff_chunk.applies(
            &LazyLines::from("B\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n"),
            0,
            false
        ),
        Some(Applies::WithReductions((1, 1)))
    );
    assert_eq!(
        diff_chunk.applies(
            &LazyLines::from("B\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n"),
            0,
            true
        ),
        Some(Applies::WithReductions((1, 1)))
    );
}

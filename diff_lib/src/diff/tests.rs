// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::*;
use crate::diff::*;
use crate::lines::*;
use crate::modifications::Modifications;

#[test]
fn diff_chunk_applies() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    let after_lines = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();

    for diff_chunk in diff_chunks.iter() {
        assert_eq!(
            diff_chunk.applies(&Lines::from(before_lines), 0, false),
            Some(Applies::Cleanly)
        );
        assert_eq!(
            diff_chunk.applies(&Lines::from(before_lines), 0, true),
            None
        );
        assert_eq!(
            diff_chunk.applies(&Lines::from(after_lines), 0, false),
            None
        );
        assert_eq!(
            diff_chunk.applies(&Lines::from(after_lines), 0, true),
            Some(Applies::Cleanly)
        );
    }

    for (i, diff_chunk) in diff_chunks.iter().enumerate() {
        assert_eq!(
            diff_chunk.applies(
                &Lines::from("a\na\na\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n"),
                3,
                false
            ),
            Some(Applies::Cleanly)
        );
        assert_eq!(
            diff_chunk.applies(
                &Lines::from("B\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n"),
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
            &Lines::from("B\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n"),
            0,
            false
        ),
        Some(Applies::WithReductions((1, 1)))
    );
    assert_eq!(
        diff_chunk.applies(
            &Lines::from("B\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n"),
            0,
            true
        ),
        Some(Applies::WithReductions((1, 1)))
    );
}

#[test]
fn find_compromise() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nT\n";
    let after_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nX\nY\nZ\n\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nT\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();

    assert_eq!(
        diff_chunks
            .first()
            .unwrap()
            .applies_nearby(&Lines::from(before_lines), 2, None, 3, false),
        Some((-3, Applies::Cleanly))
    );
    assert_eq!(
        diff_chunks
            .first()
            .unwrap()
            .applies_nearby(&Lines::from(before_lines), 2, None, -3, false),
        Some((3, Applies::Cleanly))
    );
}

#[test]
fn find_compromise_edges() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nT\n";
    let after_lines =
        "A\nX\nB\nC\nD\nE\nF\nG\nH\nI\nX\nY\nZ\n\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nX\nY\nZ\nT\n";
    let modifications = Modifications::new(Lines::from(before_lines), Lines::from(after_lines));
    let diff_chunks: Vec<ChangeChunk> = modifications.chunks::<ChangeChunk>(2).collect();
    assert_eq!(diff_chunks.len(), 3);

    assert_eq!(
        diff_chunks.first().unwrap().applies_nearby(
            &Lines::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nT\n"),
            0,
            diff_chunks.get(1),
            3,
            false
        ),
        Some((-3, Applies::Cleanly))
    );
    assert_eq!(
        diff_chunks.last().unwrap().applies_nearby(
            &Lines::from("B\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\n"),
            8,
            None,
            -3,
            false
        ),
        Some((2, Applies::WithReductions((1, 1))))
    );
}

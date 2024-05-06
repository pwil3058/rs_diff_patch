// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::apply::*;
use crate::data::*;
use crate::modifications::Modifications;
use crate::text_diff::*;

#[test]
fn diff_chunk_applies() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n";
    let after_lines = "A\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n";
    let modifications = Modifications::<String>::new(
        Data::<String>::from(before_lines),
        Data::<String>::from(after_lines),
    );
    let diff_chunks: Vec<TextChangeChunk> = modifications.chunks::<TextChangeChunk>(2).collect();

    for diff_chunk in diff_chunks.iter() {
        assert_eq!(
            diff_chunk.will_apply(&Data::<String>::from(before_lines), 0, false),
            Some(WillApply::Cleanly)
        );
        assert_eq!(
            diff_chunk.will_apply(&Data::<String>::from(before_lines), 0, true),
            None
        );
        assert_eq!(
            diff_chunk.will_apply(&Data::<String>::from(after_lines), 0, false),
            None
        );
        assert_eq!(
            diff_chunk.will_apply(&Data::<String>::from(after_lines), 0, true),
            Some(WillApply::Cleanly)
        );
    }

    for (i, diff_chunk) in diff_chunks.iter().enumerate() {
        assert_eq!(
            diff_chunk.will_apply(
                &Data::<String>::from("a\na\na\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n"),
                3,
                false
            ),
            Some(WillApply::Cleanly)
        );
        assert_eq!(
            diff_chunk.will_apply(
                &Data::<String>::from("B\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n"),
                -1,
                false
            ),
            if i > 0 {
                Some(WillApply::Cleanly)
            } else {
                Some(WillApply::WithReductions((1, 1)))
            }
        );
    }

    let diff_chunk = diff_chunks.first().unwrap();
    assert_eq!(
        diff_chunk.will_apply(
            &Data::<String>::from("B\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\n"),
            0,
            false
        ),
        Some(WillApply::WithReductions((1, 1)))
    );
    assert_eq!(
        diff_chunk.will_apply(
            &Data::<String>::from("B\nC\nD\nEf\nFg\nG\nH\nI\nJ\nK\nH\nL\nM\n"),
            0,
            true
        ),
        Some(WillApply::WithReductions((1, 1)))
    );
}

#[test]
fn find_compromise() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nT\n";
    let after_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nX\nY\nZ\n\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nT\n";
    let modifications = Modifications::<String>::new(
        Data::<String>::from(before_lines),
        Data::<String>::from(after_lines),
    );
    let diff_chunks: Vec<TextChangeChunk> = modifications.chunks::<TextChangeChunk>(2).collect();
    let lines = Data::<String>::from(before_lines);
    let mut pd = PatchableData::new(&lines);
    pd.advance_consumed_by(2);

    assert_eq!(
        diff_chunks
            .first()
            .unwrap()
            .will_apply_nearby(&pd, None, 3, false),
        Some((-3, WillApply::Cleanly))
    );

    assert_eq!(
        diff_chunks
            .first()
            .unwrap()
            .will_apply_nearby(&pd, None, -3, false),
        Some((3, WillApply::Cleanly))
    );
}

#[test]
fn find_compromise_edges() {
    let before_lines = "A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nT\n";
    let after_lines =
        "A\nX\nB\nC\nD\nE\nF\nG\nH\nI\nX\nY\nZ\n\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nX\nY\nZ\nT\n";
    let modifications = Modifications::<String>::new(
        Data::<String>::from(before_lines),
        Data::<String>::from(after_lines),
    );
    let diff_chunks: Vec<TextChangeChunk> = modifications.chunks::<TextChangeChunk>(2).collect();

    assert_eq!(diff_chunks.len(), 3);

    let lines = Data::<String>::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\nT\n");
    let pd = PatchableData::new(&lines);
    assert_eq!(
        diff_chunks
            .first()
            .unwrap()
            .will_apply_nearby(&pd, diff_chunks.get(1), 3, false),
        Some((-3, WillApply::Cleanly))
    );

    let lines = Data::<String>::from("B\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nO\nP\nQ\nR\nS\n");
    let mut pd = PatchableData::new(&lines);
    pd.advance_consumed_by(8);
    assert_eq!(
        diff_chunks
            .last()
            .unwrap()
            .will_apply_nearby(&pd, None, -3, false),
        Some((2, WillApply::WithReductions((1, 1))))
    );
}

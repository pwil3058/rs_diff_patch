// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::unified_diffs::UnifiedDiffs;

#[test]
fn minimal() {
    let diffs = UnifiedDiffs::new("---\n+++\n");
    assert_eq!(diffs.preamble, None);
    assert_eq!("---\n", diffs.diffs.first().unwrap().before);
    let diffs = UnifiedDiffs::new("blah blah blah\n---\n+++\n");
    assert_eq!(diffs.preamble.unwrap(), "blah blah blah");
    assert_eq!("---\n", diffs.diffs.first().unwrap().before);
}

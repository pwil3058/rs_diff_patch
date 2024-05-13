// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::common_subsequence::CommonSubsequence;
use crate::range::Range;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Change {
    NoChange(CommonSubsequence),
    Delete(Range, usize),
    Insert(usize, Range),
    Replace(Range, Range),
}

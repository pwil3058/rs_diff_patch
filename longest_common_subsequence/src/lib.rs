// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

pub mod common_subsequence;
pub mod range;
pub mod sequence;
pub mod snippet;

use common_subsequence::CommonSubsequence;
use sequence::Seq;

pub fn longest_common_subsequence<T: PartialEq + Clone>(
    _left: &Seq<T>,
    _right: &Seq<T>,
) -> Option<CommonSubsequence> {
    None
}

pub fn longest_common_subsequences<T: PartialEq + Clone>(
    _left: &Seq<T>,
    _right: &Seq<T>,
) -> Seq<CommonSubsequence> {
    Seq(vec![].into_boxed_slice())
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}

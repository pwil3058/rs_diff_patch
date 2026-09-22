// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

pub mod common_subsequence;
mod lcs;
pub mod range;
pub mod sequence;
pub mod snippet;

use common_subsequence::CommonSubsequence;
use sequence::Seq;

/// Find the longest common subsequences in the given sequences
///
/// Example:
/// ```
/// use longest_common_subsequence::sequence::Seq;
/// use longest_common_subsequence::common_subsequence::CommonSubsequence;
/// use longest_common_subsequence::longest_common_subsequence;
/// let left = Seq::<String>("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\n"
///     .split_inclusive('\n').map(|s| s.to_string()).collect::<Vec<_>>().into_boxed_slice());
/// let right = Seq::<String>("X\nY\nZ\nC\nD\nE\nH\nI\nX\n"
///     .split_inclusive('\n').map(|s| s.to_string()).collect::<Vec<_>>().into_boxed_slice());
/// assert_eq!(Some(CommonSubsequence(2,3,3)), longest_common_subsequence(&left, &right));
/// ```
pub fn longest_common_subsequence<T: PartialEq + Eq + Clone + std::hash::Hash>(
    left: &Seq<T>,
    right: &Seq<T>,
) -> Option<CommonSubsequence> {
    let data = lcs::Data::<T>::new(left, right);
    data.longest_common_subsequence(left.range_from(0), right.range_from(0))
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

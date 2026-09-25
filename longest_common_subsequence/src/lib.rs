// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

pub mod changes;
pub mod common_subsequence;
mod lcs;
pub mod range;
pub mod sequence;

use common_subsequence::CommonSubsequence;
use sequence::Seq;

/// Find the longest common subsequence in the given sequences
///
/// Example:
/// ```
/// use longest_common_subsequence::sequence::Seq;
/// use longest_common_subsequence::common_subsequence::CommonSubsequence;
/// use longest_common_subsequence::longest_common_subsequence;
/// let left = Seq::<String>::from("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\n"
///     .split_inclusive('\n').map(|s| s.to_string()).collect::<Vec<_>>());
/// let right = Seq::<String>::from_iter("X\nY\nZ\nC\nD\nE\nH\nI\nX\n"
///     .split_inclusive('\n').map(|s| s.to_string()));
/// assert_eq!(Some(CommonSubsequence(2,3,3)), longest_common_subsequence(&left, &right));
/// ```
pub fn longest_common_subsequence<T: PartialEq + Eq + Clone + std::hash::Hash>(
    left: &Seq<T>,
    right: &Seq<T>,
) -> Option<CommonSubsequence> {
    let data = lcs::Data::<T>::new(left, right);
    data.longest_common_subsequence(left.range_from(0), right.range_from(0))
}

/// Find the longest common subsequences in the given sequences
///
/// Example:
/// ```
/// use longest_common_subsequence::sequence::Seq;
/// use longest_common_subsequence::common_subsequence::CommonSubsequence;
/// use longest_common_subsequence::longest_common_subsequences;
/// let left = Seq::<String>("A\nB\nC\nD\nE\nF\nG\nH\nI\nJ\n"
///     .split_inclusive('\n').map(|s| s.to_string()).collect::<Vec<_>>().into_boxed_slice());
/// let right = Seq::<String>("X\nY\nZ\nC\nD\nE\nH\nI\nX\n"
///     .split_inclusive('\n').map(|s| s.to_string()).collect::<Vec<_>>().into_boxed_slice());
/// assert_eq!(
///     Seq(vec![CommonSubsequence(2,3,3),CommonSubsequence(7, 6, 2)].into_boxed_slice()),
///     longest_common_subsequences(&left, &right)
/// );
/// ```
pub fn longest_common_subsequences<T: PartialEq + Eq + Clone + std::hash::Hash>(
    left: &Seq<T>,
    right: &Seq<T>,
) -> Seq<CommonSubsequence> {
    let data = lcs::Data::<T>::new(left, right);
    Seq(data.longest_common_subsequences().into_boxed_slice())
}

// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use serde::{Deserialize, Serialize};
use std::collections::Bound;
use std::ops::RangeBounds;

pub trait Len {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct CRange(pub usize, pub usize);

impl Len for CRange {
    fn len(&self) -> usize {
        if self.1 > self.0 {
            self.1 - self.0
        } else {
            0
        }
    }
}

impl CRange {
    pub fn start(&self) -> usize {
        self.0
    }

    pub fn end(&self) -> usize {
        self.1
    }

    pub fn from(bounds: impl RangeBounds<usize>) -> Self {
        let start = match bounds.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i + 1,
            _ => 0,
        };
        let end = match bounds.end_bound() {
            Bound::Included(i) => *i + 1,
            Bound::Excluded(i) => *i,
            _ => usize::MAX,
        };

        CRange(start, end)
    }
}

impl RangeBounds<usize> for CRange {
    fn start_bound(&self) -> Bound<&usize> {
        Bound::Included(&self.0)
    }

    fn end_bound(&self) -> Bound<&usize> {
        Bound::Excluded(&self.1)
    }
}

#[cfg(test)]
mod crange_tests {

    #[test]
    fn crange() {
        let crange = CRange(3, 5);
        assert_eq!(crange.start(), 3);
        assert_eq!(crange.end(), 5);
        assert_eq!(crange.len(), 2);
        assert_eq!(CRange::from(..), CRange(0, usize::MAX));
        assert_eq!(CRange::from(1..6), CRange(1, 6));
    }
    use super::*;
}

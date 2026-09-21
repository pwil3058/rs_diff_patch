// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use serde::{Deserialize, Serialize};

pub trait Len {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq, Serialize, Deserialize)]
pub struct Range(pub usize, pub usize);

impl Len for Range {
    fn len(&self) -> usize {
        self.1.saturating_sub(self.0)
    }
}

impl Range {
    pub fn start(&self) -> usize {
        self.0
    }

    pub fn end(&self) -> usize {
        self.1
    }

    pub fn is_valid(&self) -> bool {
        self.0 <= self.1
    }

    pub fn is_valid_for_max_end(&self, max_end: usize) -> bool {
        self.is_valid() && self.1 <= max_end
    }
}

#[cfg(test)]
mod range_tests {
    use super::*;

    #[test]
    fn crange() {
        let crange = Range(3, 5);
        assert_eq!(crange.start(), 3);
        assert_eq!(crange.end(), 5);
        assert_eq!(crange.len(), 2);
    }

    #[test]
    fn valid() {
        assert!(Range(2, 3).is_valid());
        assert!(!Range(3, 10).is_valid_for_max_end(9));
        assert!(Range(3, 10).is_valid_for_max_end(10));
        assert!(Range(3, 10).is_valid_for_max_end(11));
    }
}

// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

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
        if self.1 > self.0 {
            self.1 - self.0
        } else {
            0
        }
    }
}

impl Range {
    pub fn start(&self) -> usize {
        self.0
    }

    pub fn end(&self) -> usize {
        self.1
    }
}

#[cfg(test)]
mod crange_tests {

    #[test]
    fn crange() {
        let crange = Range(3, 5);
        assert_eq!(crange.start(), 3);
        assert_eq!(crange.end(), 5);
        assert_eq!(crange.len(), 2);
    }
    use super::*;
}

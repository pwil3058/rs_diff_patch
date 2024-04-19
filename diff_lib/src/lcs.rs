// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::crange::{CRange, Len};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Serialize, Deserialize)]
pub struct LongestCommonSubsequence(pub usize, pub usize, pub usize);

impl Len for LongestCommonSubsequence {
    fn len(&self) -> usize {
        self.2
    }
}

impl LongestCommonSubsequence {
    pub fn antemod_range(&self) -> CRange {
        CRange(self.0, self.0 + self.2)
    }

    pub fn postmod_range(&self) -> CRange {
        CRange(self.1, self.1 + self.2)
    }

    pub fn antemod_start(&self) -> usize {
        self.0
    }

    pub fn postmod_start(&self) -> usize {
        self.1
    }

    pub fn antemod_end(&self) -> usize {
        self.0 + self.2
    }

    pub fn postmod_end(&self) -> usize {
        self.1 + self.2
    }

    pub fn decr_starts(&mut self, arg: usize) {
        self.0 -= arg;
        self.1 -= arg;
        self.2 += arg;
    }

    pub fn incr_starts(&mut self, arg: usize) {
        self.0 += arg;
        self.1 += arg;
        self.2 -= arg;
    }

    pub fn starts_trimmed(&self, arg: usize) -> Self {
        if self.2 > arg {
            Self(self.0 + self.2 - arg, self.1 + self.2 - arg, arg)
        } else {
            *self
        }
    }

    pub fn ends_trimmed(&self, arg: usize) -> Self {
        if self.2 > arg {
            Self(self.0, self.1, arg)
        } else {
            *self
        }
    }

    pub fn split(&self, arg: usize) -> Option<(Self, Self)> {
        if self.2 >= arg * 2 {
            Some((self.ends_trimmed(arg), self.starts_trimmed(arg)))
        } else {
            None
        }
    }

    pub fn incr_size(&mut self, arg: usize) {
        self.2 += arg;
    }

    pub fn decr_size(&mut self, arg: usize) {
        self.2 -= arg;
    }
}

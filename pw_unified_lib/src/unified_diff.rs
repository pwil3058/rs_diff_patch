// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::unified_parser::AATerminal;

#[derive(Debug, Default, Clone)]
pub struct UnifiedDiff {
    _header: String,
    _lines: Box<[String]>,
}

#[derive(Debug, Default, Clone)]
pub struct UnifiedDiffs {
    preamble: Option<String>,
    _diffs: Vec<UnifiedDiff>,
}

impl UnifiedDiffs {
    pub fn set_preamble(&mut self, preamble: &str) {
        self.preamble = Some(preamble.to_string())
    }
}

impl lalr1::ReportError<AATerminal> for UnifiedDiffs {}

#[cfg(test)]
mod diff_tests {
    fn _unified_diff() {
        assert!(true)
    }
}

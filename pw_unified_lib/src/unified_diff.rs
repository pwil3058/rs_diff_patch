// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::unified_parser::AATerminal;
use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub struct UnifiedDiff {
    pub before_path: PathBuf,
    pub after_path: PathBuf,
    pub header: String,
    pub lines: Box<[String]>,
}

#[derive(Debug, Default, Clone)]
pub struct UnifiedDiffPatch;

impl lalr1::ReportError<AATerminal> for UnifiedDiffPatch {}

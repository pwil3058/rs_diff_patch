// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::unified_parser::AATerminal;

#[derive(Debug, Default, Clone)]
pub struct UnifiedDiff;

#[derive(Debug, Default, Clone)]
pub struct UnifiedDiffPatch;

impl lalr1::ReportError<AATerminal> for UnifiedDiffPatch {}

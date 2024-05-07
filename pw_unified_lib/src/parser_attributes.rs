// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::unified_diff::UnifiedDiff;
use crate::unified_parser::AATerminal;

#[derive(Debug, Default, Clone)]
pub enum ParserAttributes {
    Token(lexan::Token<AATerminal>),
    Error(lalr1::Error<AATerminal>),
    BeforePath(String),
    AfterPath(String),
    ChunkHeader(String),
    ChunkLine(String),
    ChunkLines(Vec<String>),
    Diff(UnifiedDiff),
    DiffList(Vec<UnifiedDiff>),
    #[default]
    Default,
}

impl ParserAttributes {
    pub fn before_path(&self) -> &String {
        match self {
            ParserAttributes::BeforePath(path) => path,
            _ => panic!("invalid variant"),
        }
    }

    pub fn chunk_line(&self) -> &String {
        match self {
            ParserAttributes::ChunkLine(line) => line,
            _ => panic!("invalid variant"),
        }
    }

    pub fn chunk_lines_mut(&mut self) -> &mut Vec<String> {
        match self {
            ParserAttributes::ChunkLines(list) => list,
            _ => panic!("{self:?}: Wrong attribute variant."),
        }
    }

    pub fn diff(&self) -> &UnifiedDiff {
        match self {
            ParserAttributes::Diff(diff) => diff,
            _ => panic!("{self:?}: Wrong attribute variant."),
        }
    }

    pub fn diff_mut(&mut self) -> &mut Vec<UnifiedDiff> {
        match self {
            ParserAttributes::DiffList(list) => list,
            _ => panic!("{self:?}: Wrong attribute variant."),
        }
    }
}

impl From<lexan::Token<AATerminal>> for ParserAttributes {
    fn from(input: lexan::Token<AATerminal>) -> Self {
        match input.tag() {
            AATerminal::BeforePath => ParserAttributes::BeforePath(input.lexeme().to_string()),
            AATerminal::AfterPath => ParserAttributes::AfterPath(input.lexeme().to_string()),
            AATerminal::ChunkHeader => ParserAttributes::ChunkHeader(input.lexeme().to_string()),
            AATerminal::ChunkLine => ParserAttributes::ChunkLine(input.lexeme().to_string()),
            _ => ParserAttributes::Token(input.clone()),
        }
    }
}

impl From<lalr1::Error<AATerminal>> for ParserAttributes {
    fn from(error: lalr1::Error<AATerminal>) -> Self {
        ParserAttributes::Error(error.clone())
    }
}

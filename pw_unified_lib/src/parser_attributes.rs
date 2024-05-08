// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::unified_diff::AATerminal;
use crate::unified_diff::UnifiedDiff;

#[derive(Debug, Default, Clone)]
pub enum ParserAttributes {
    String(String),
    Strings(Vec<String>),
    Diff(UnifiedDiff),
    Diffs(Vec<UnifiedDiff>),
    Token(lexan::Token<AATerminal>),
    Error(lalr1::Error<AATerminal>),
    #[default]
    Default,
}

impl From<lexan::Token<AATerminal>> for ParserAttributes {
    fn from(token: lexan::Token<AATerminal>) -> Self {
        ParserAttributes::Token(token)
    }
}

impl ParserAttributes {
    pub fn token(&self) -> &lexan::Token<AATerminal> {
        match self {
            ParserAttributes::Token(token) => token,
            _ => panic!("invalid variant"),
        }
    }
    // pub fn string(&self) -> &String {
    //     match self {
    //         ParserAttributes::String(string) => string,
    //         _ => panic!("invalid variant"),
    //     }
    // }
    //
    pub fn strings_mut(&mut self) -> &mut Vec<String> {
        match self {
            ParserAttributes::Strings(strings) => strings,
            _ => panic!("{self:?}: Wrong attribute variant."),
        }
    }
    //
    // pub fn diff(&self) -> &UnifiedDiff {
    //     match self {
    //         ParserAttributes::Diff(diff) => diff,
    //         _ => panic!("{self:?}: Wrong attribute variant."),
    //     }
    // }
    //
    // pub fn diffs_mut(&mut self) -> &mut Vec<UnifiedDiff> {
    //     match self {
    //         ParserAttributes::Diffs(diffs) => diffs,
    //         _ => panic!("{self:?}: Wrong attribute variant."),
    //     }
    // }
}

// impl From<lexan::Token<AATerminal>> for ParserAttributes {
//     fn from(input: lexan::Token<AATerminal>) -> Self {
//         use AATerminal::*;
//         match input.tag() {
//             BeforePath | AfterPath | ChunkHeader | ChunkLine | Preamble => {
//                 ParserAttributes::String(input.lexeme().to_string())
//             }
//             _ => ParserAttributes::Token(input.clone()),
//         }
//     }
// }

impl From<lalr1::Error<AATerminal>> for ParserAttributes {
    fn from(error: lalr1::Error<AATerminal>) -> Self {
        ParserAttributes::Error(error.clone())
    }
}

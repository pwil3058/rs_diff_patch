// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::unified_diffs::AATerminal;
use std::collections::BTreeSet;

#[derive(Debug, Default, Clone)]
pub enum ParserAttributes {
    Token(lexan::Token<AATerminal>),
    SyntaxError(lexan::Token<AATerminal>, BTreeSet<AATerminal>),
    LexicalError(lexan::Error<AATerminal>, BTreeSet<AATerminal>),
    #[default]
    Default,
}

impl From<lexan::Token<AATerminal>> for ParserAttributes {
    fn from(token: lexan::Token<AATerminal>) -> Self {
        ParserAttributes::Token(token)
    }
}

impl From<lalr1::Error<AATerminal>> for ParserAttributes {
    fn from(error: lalr1::Error<AATerminal>) -> Self {
        match error {
            lalr1::Error::LexicalError(error, expected) => {
                ParserAttributes::LexicalError(error, expected)
            }
            lalr1::Error::SyntaxError(token, expected) => {
                ParserAttributes::SyntaxError(token, expected)
            }
        }
    }
}

impl ParserAttributes {
    pub fn token(&self) -> &lexan::Token<AATerminal> {
        match self {
            ParserAttributes::Token(token) => token,
            _ => panic!("invalid variant"),
        }
    }
}

// impl From<lalr1::Error<AATerminal>> for ParserAttributes {
//     fn from(error: lalr1::Error<AATerminal>) -> Self {
//         match error {
//             lalr1::Error::LexicalError(error, expected) => {
//                 ParserAttributes::LexicalError(error, expected)
//             }
//             lalr1::Error::SyntaxError(token, expected) => {
//                 ParserAttributes::SyntaxError(token, expected)
//             }
//         }
//     }
// }

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

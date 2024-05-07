// generated by lap_gen.

use crate::parser_attributes::ParserAttributes;
use crate::unified_diff::{UnifiedDiff, UnifiedDiffPatch};

use lazy_static::lazy_static;
use std::collections::BTreeSet;

macro_rules! btree_set {
    () => { BTreeSet::new() };
    ( $( $x:expr ),* ) => {
        {
            let mut set = BTreeSet::new();
            $( set.insert($x); )*
            set
        }
    };
    ( $( $x:expr ),+ , ) => {
        btree_set![ $( $x ), * ]
    };
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub enum AATerminal {
    AAEnd,
    AfterPath,
    BeforePath,
    ChunkHeader,
    ChunkLine,
    Preamble,
}

impl std::fmt::Display for AATerminal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AATerminal::AAEnd => write!(f, r###"AAEnd"###),
            AATerminal::AfterPath => write!(f, r###"AfterPath"###),
            AATerminal::BeforePath => write!(f, r###"BeforePath"###),
            AATerminal::ChunkHeader => write!(f, r###"ChunkHeader"###),
            AATerminal::ChunkLine => write!(f, r###"ChunkLine"###),
            AATerminal::Preamble => write!(f, r###"Preamble"###),
        }
    }
}

lazy_static! {
    static ref AALEXAN: lexan::LexicalAnalyzer<AATerminal> = {
        use AATerminal::*;
        lexan::LexicalAnalyzer::new(
            &[],
            &[
                (Preamble, r###"([^-]{3}*)"###),
                (BeforePath, r###"(^---.*\n)"###),
                (ChunkHeader, r###"(^@@.*@@.*\n)"###),
                (ChunkLine, r###"(^[ -+].*\n)"###),
                (AfterPath, r###"(^\+\+\+.*\n)"###),
            ],
            &[],
            AAEnd,
        )
    };
}

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub enum AANonTerminal {
    AAStart,
    ChunkLines,
    Diff,
    DiffList,
    Specification,
}

impl std::fmt::Display for AANonTerminal {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AANonTerminal::AAStart => write!(f, r"AAStart"),
            AANonTerminal::ChunkLines => write!(f, r"ChunkLines"),
            AANonTerminal::Diff => write!(f, r"Diff"),
            AANonTerminal::DiffList => write!(f, r"DiffList"),
            AANonTerminal::Specification => write!(f, r"Specification"),
        }
    }
}

impl lalr1::Parser<AATerminal, AANonTerminal, ParserAttributes> for UnifiedDiffPatch {
    fn lexical_analyzer(&self) -> &lexan::LexicalAnalyzer<AATerminal> {
        &AALEXAN
    }

    fn viable_error_recovery_states(_token: &AATerminal) -> BTreeSet<u32> {
        btree_set![]
    }

    fn look_ahead_set(state: u32) -> BTreeSet<AATerminal> {
        use AATerminal::*;
        return match state {
            0 => btree_set![Preamble],
            1 => btree_set![AAEnd],
            2 => btree_set![BeforePath],
            3 => btree_set![BeforePath, AAEnd],
            4 => btree_set![BeforePath, AAEnd],
            5 => btree_set![AfterPath],
            6 => btree_set![BeforePath, AAEnd],
            7 => btree_set![ChunkHeader],
            8 => btree_set![ChunkLine],
            9 => btree_set![BeforePath, ChunkLine, AAEnd],
            10 => btree_set![BeforePath, ChunkLine, AAEnd],
            11 => btree_set![BeforePath, ChunkLine, AAEnd],
            _ => panic!("illegal state: {state}"),
        };
    }

    fn next_action(&self, aa_state: u32, aa_token: &lexan::Token<AATerminal>) -> lalr1::Action {
        use lalr1::Action;
        use AATerminal::*;
        let aa_tag = *aa_token.tag();
        return match aa_state {
            0 => match aa_tag {
                Preamble => Action::Shift(2),
                _ => Action::SyntaxError,
            },
            1 => match aa_tag {
                // AAStart: Specification #(NonAssoc, 0)
                AAEnd => Action::Accept,
                _ => Action::SyntaxError,
            },
            2 => match aa_tag {
                BeforePath => Action::Shift(5),
                _ => Action::SyntaxError,
            },
            3 => match aa_tag {
                BeforePath => Action::Shift(5),
                // Specification: Preamble DiffList #(NonAssoc, 0)
                AAEnd => Action::Reduce(1),
                _ => Action::SyntaxError,
            },
            4 => match aa_tag {
                // DiffList: Diff #(NonAssoc, 0)
                BeforePath | AAEnd => Action::Reduce(2),
                _ => Action::SyntaxError,
            },
            5 => match aa_tag {
                AfterPath => Action::Shift(7),
                _ => Action::SyntaxError,
            },
            6 => match aa_tag {
                // DiffList: DiffList Diff #(NonAssoc, 0)
                BeforePath | AAEnd => Action::Reduce(3),
                _ => Action::SyntaxError,
            },
            7 => match aa_tag {
                ChunkHeader => Action::Shift(8),
                _ => Action::SyntaxError,
            },
            8 => match aa_tag {
                ChunkLine => Action::Shift(10),
                _ => Action::SyntaxError,
            },
            9 => match aa_tag {
                ChunkLine => Action::Shift(11),
                // Diff: BeforePath AfterPath ChunkHeader ChunkLines #(NonAssoc, 0)
                BeforePath | AAEnd => Action::Reduce(4),
                _ => Action::SyntaxError,
            },
            10 => match aa_tag {
                // ChunkLines: ChunkLine #(NonAssoc, 0)
                BeforePath | ChunkLine | AAEnd => Action::Reduce(5),
                _ => Action::SyntaxError,
            },
            11 => match aa_tag {
                // ChunkLines: ChunkLines ChunkLine #(NonAssoc, 0)
                BeforePath | ChunkLine | AAEnd => Action::Reduce(6),
                _ => Action::SyntaxError,
            },
            _ => panic!("illegal state: {aa_state}"),
        };
    }

    fn production_data(production_id: u32) -> (AANonTerminal, usize) {
        match production_id {
            0 => (AANonTerminal::AAStart, 1),
            1 => (AANonTerminal::Specification, 2),
            2 => (AANonTerminal::DiffList, 1),
            3 => (AANonTerminal::DiffList, 2),
            4 => (AANonTerminal::Diff, 4),
            5 => (AANonTerminal::ChunkLines, 1),
            6 => (AANonTerminal::ChunkLines, 2),
            _ => panic!("malformed production data table"),
        }
    }

    fn goto_state(lhs: &AANonTerminal, current_state: u32) -> u32 {
        return match current_state {
            0 => match lhs {
                AANonTerminal::Specification => 1,
                _ => panic!("Malformed goto table: ({lhs}, {current_state})"),
            },
            2 => match lhs {
                AANonTerminal::Diff => 4,
                AANonTerminal::DiffList => 3,
                _ => panic!("Malformed goto table: ({lhs}, {current_state})"),
            },
            3 => match lhs {
                AANonTerminal::Diff => 6,
                _ => panic!("Malformed goto table: ({lhs}, {current_state})"),
            },
            8 => match lhs {
                AANonTerminal::ChunkLines => 9,
                _ => panic!("Malformed goto table: ({lhs}, {current_state})"),
            },
            _ => panic!("Malformed goto table: ({lhs}, {current_state})"),
        };
    }

    fn do_semantic_action<F: FnMut(String, String)>(
        &mut self,
        aa_production_id: u32,
        aa_rhs: Vec<ParserAttributes>,
        mut aa_inject: F,
    ) -> ParserAttributes {
        let mut aa_lhs = if let Some(a) = aa_rhs.first() {
            a.clone()
        } else {
            ParserAttributes::default()
        };
        match aa_production_id {
            2 => {
                // DiffList: Diff #(NonAssoc, 0)

                aa_lhs = ParserAttributes::DiffList(vec![aa_rhs[0].diff().clone()]);
            }
            3 => {
                // DiffList: DiffList Diff #(NonAssoc, 0)

                aa_lhs.diff_mut().push(aa_rhs[1].diff().clone());
            }
            5 => {
                // ChunkLines: ChunkLine #(NonAssoc, 0)

                aa_lhs = ParserAttributes::ChunkLines(vec![aa_rhs[0].chunk_line().clone()]);
            }
            6 => {
                // ChunkLines: ChunkLines ChunkLine #(NonAssoc, 0)

                aa_lhs
                    .chunk_lines_mut()
                    .push(aa_rhs[1].chunk_line().clone());
            }
            _ => aa_inject(String::new(), String::new()),
        };
        aa_lhs
    }
}
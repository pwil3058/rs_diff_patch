// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
pub enum DiffParseError {
    ParseNumberError(ParseIntError, usize),
    UnexpectedEndOfInput,
    UnexpectedEndClump(usize),
    SyntaxError(usize),
}

impl fmt::Display for DiffParseError {
    // TODO: flesh out fmt::Display implementation for DiffParseError
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bug the developer to fix this!")
    }
}

impl Error for DiffParseError {}

pub type DiffParseResult<T> = Result<T, DiffParseError>;

pub trait CheckEndOfInput<T> {
    fn check_end_of_input(&self) -> DiffParseResult<&T>;
}

impl<T> CheckEndOfInput<T> for Option<T> {
    fn check_end_of_input(&self) -> DiffParseResult<&T> {
        match self {
            Some(t) => Ok(t),
            None => Err(DiffParseError::UnexpectedEndOfInput),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PathAndTimestamp {
    pub file_path: String,
    pub time_stamp: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct StartAndLength {
    pub start: usize,
    pub length: usize,
}

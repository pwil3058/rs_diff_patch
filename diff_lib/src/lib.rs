// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

pub mod apply;
pub mod diff;
pub mod lcs;
pub mod lines;
pub mod modifications;
pub mod range;
pub mod snippet;

pub use crate::{diff::*, lines::*, modifications::*};

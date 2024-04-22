// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::lines::{BasicLines, LazyLines};
use serde::{Deserialize, Serialize};
use std::io;

pub struct ProgressData<'a, L>
where
    L: BasicLines,
{
    pub lines: &'a L,
    pub consumed: usize,
    pub offset: isize,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Applies {
    Cleanly,
    WithReductions((usize, usize)),
}

pub trait MatchesAt: BasicLines {
    fn matches_at(&self, lines: &[String], at: usize) -> bool {
        if at < self.len() && self.len() - at >= lines.len() {
            lines.iter().zip(self.lines(at..)).all(|(b, a)| a == *b)
        } else {
            false
        }
    }
}

impl MatchesAt for LazyLines {}

pub trait ApplyChunkInto {
    fn antemodn_lines(
        &self,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> impl Iterator<Item = &String>;
    fn postmodn_lines(
        &self,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> impl Iterator<Item = &String>;
    fn applies(&self, lines: &impl MatchesAt, offset: isize, reverse: bool) -> Option<Applies>;
    fn applies_nearby(
        &self,
        lines: &impl MatchesAt,
        not_before: usize,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, Applies)>;
    fn apply_into<'a, L, W>(
        &self,
        pd: &mut ProgressData<'a, L>,
        into: &mut W,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> io::Result<()>
    where
        L: BasicLines,
        W: io::Write;
    fn already_applied_into<'a, L, W>(
        &self,
        pd: &mut ProgressData<'a, L>,
        into: &mut W,
        reductions: Option<(usize, usize)>,
        reverse: bool,
    ) -> io::Result<()>
    where
        L: BasicLines,
        W: io::Write;
}

pub trait ApplyInto<'a, C: ApplyChunkInto>: Serialize + Deserialize<'a> {
    fn chunks<'s>(&'s self) -> impl Iterator<Item = &'s C>
    where
        C: 's;

    fn apply_into<W>(&self, target: &impl MatchesAt, into: &mut W, reverse: bool) -> io::Result<()>
    where
        W: io::Write,
    {
        let mut pd = ProgressData {
            lines: target,
            consumed: 0,
            offset: 0,
        };
        let mut iter = self.chunks().peekable();
        let mut chunk_num = 0;
        while let Some(chunk) = iter.next() {
            chunk_num += 1; // for human consumption
            if pd.consumed > target.len() {
                log::error!("Unexpected end of input processing hunk #{chunk_num}.");
            }
            if let Some(applies) = chunk.applies(target, pd.offset, reverse) {
                match applies {
                    Applies::Cleanly => {
                        chunk.apply_into(&mut pd, into, None, reverse)?;
                        log::info!("Chunk #{chunk_num} applies cleanly.");
                    }
                    Applies::WithReductions(reductions) => {
                        chunk.apply_into(&mut pd, into, Some(reductions), reverse)?;
                        log::warn!("Chunk #{chunk_num} applies with {reductions:?} reductions.");
                    }
                }
            } else if let Some((offset_adj, applies)) = chunk.applies_nearby(
                target,
                pd.consumed,
                iter.peek().cloned(),
                pd.offset,
                reverse,
            ) {
                pd.offset += offset_adj;
                match applies {
                    Applies::Cleanly => {
                        chunk.apply_into(&mut pd, into, None, reverse)?;
                        log::warn!("Chunk #{chunk_num} applies with offset {offset_adj}.");
                    }
                    Applies::WithReductions(reductions) => {
                        chunk.apply_into(&mut pd, into, Some(reductions), reverse)?;
                        log::warn!("Chunk #{chunk_num} applies with {reductions:?} reductions and offset {offset_adj}.");
                    }
                }
            } else if let Some(applies) = chunk.applies(target, pd.offset, !reverse) {
                // It's already applied
                match applies {
                    Applies::Cleanly => {
                        chunk.already_applied_into(&mut pd, into, None, reverse)?;
                        log::warn!("Chunk #{chunk_num} already applied")
                    }
                    Applies::WithReductions(reductions) => {
                        chunk.already_applied_into(&mut pd, into, Some(reductions), reverse)?;
                        log::warn!(
                            "Chunk #{chunk_num} already applied with {reductions:?} reductions."
                        );
                    }
                }
            } else if let Some((offset_adj, applies)) = chunk.applies_nearby(
                target,
                pd.consumed,
                iter.peek().cloned(),
                pd.offset,
                !reverse,
            ) {
                // It's already applied
                pd.offset += offset_adj;
                match applies {
                    Applies::Cleanly => {
                        chunk.already_applied_into(&mut pd, into, None, reverse)?;
                        log::warn!("Chunk #{chunk_num} already applied with offset {offset_adj}")
                    }
                    Applies::WithReductions(reductions) => {
                        chunk.already_applied_into(&mut pd, into, Some(reductions), reverse)?;
                        log::warn!("Chunk #{chunk_num} already applied with {reductions:?} reductions and offset {offset_adj}.")
                    }
                }
            } else {
                into.write_all(b"<<<<<<<\n")?;
                for line in chunk.antemodn_lines(None, reverse) {
                    into.write_all(line.as_bytes())?;
                }
                into.write_all(b"=======\n")?;
                for line in chunk.postmodn_lines(None, reverse) {
                    into.write_all(line.as_bytes())?;
                }
                into.write_all(b">>>>>>>\n")?;
                log::error!("Chunk #{chunk_num} could NOT be applied!");
            }
        }
        for line in pd.lines.lines(pd.consumed..) {
            into.write_all(line.as_bytes())?;
        }
        Ok(())
    }
}

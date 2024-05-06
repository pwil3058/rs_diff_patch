// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::data::{DataIfce, WriteDataInto};
use crate::range::Range;
use std::cmp::Ordering;
use std::io;
use std::marker::PhantomData;

use log;

#[derive(Debug, Clone)]
pub struct PatchableData<'a, T, D>
where
    T: PartialEq + Clone,
    D: DataIfce<T> + WriteDataInto + Clone,
{
    data: &'a D,
    consumed: usize,
    phantom_data: PhantomData<&'a T>,
}

pub trait PatchableDataIfce<'a, T: PartialEq, D: DataIfce<T>>
where
    D: WriteDataInto,
{
    fn new(data: &'a D) -> Self;
    fn data(&self) -> &D;
    fn consumed(&self) -> usize;
    fn range_from(&self, from: usize) -> Range;
    fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool;
    fn advance_consumed_by(&mut self, increment: usize);
    fn write_into_upto<W: io::Write>(&mut self, writer: &mut W, upto: usize) -> io::Result<bool>;
    fn write_remainder<W: io::Write>(&mut self, writer: &mut W) -> io::Result<bool>;
}

impl<'a, T: PartialEq + Clone, D: DataIfce<T> + WriteDataInto + Clone> PatchableDataIfce<'a, T, D>
    for PatchableData<'a, T, D>
{
    fn new(data: &'a D) -> Self {
        Self {
            data,
            consumed: 0,
            phantom_data: PhantomData,
        }
    }

    #[inline]
    fn data(&self) -> &D {
        self.data
    }

    #[inline]
    fn consumed(&self) -> usize {
        self.consumed
    }

    fn range_from(&self, from: usize) -> Range {
        Range(from, self.data.len())
    }

    #[inline]
    fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool {
        self.data.has_subsequence_at(subsequence, at)
    }

    fn advance_consumed_by(&mut self, increment: usize) {
        self.consumed += increment
    }

    fn write_into_upto<W: io::Write>(&mut self, writer: &mut W, upto: usize) -> io::Result<bool> {
        if upto <= self.data.len() {
            match self.consumed.cmp(&upto) {
                Ordering::Less => {
                    let range = Range(self.consumed, upto);
                    self.consumed = upto;
                    self.data.write_into(writer, range)
                }
                Ordering::Equal => Ok(true),
                Ordering::Greater => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
    fn write_remainder<W: io::Write>(&mut self, writer: &mut W) -> io::Result<bool> {
        let range = self.range_from(self.consumed);
        self.consumed = self.data.len();
        self.data.write_into(writer, range)
    }
}

pub trait ApplyChunkClean<T, D>
where
    T: PartialEq + Clone,
    D: DataIfce<T> + WriteDataInto + Clone,
{
    fn will_apply(&self, data: &D, reverse: bool) -> bool;
    fn is_already_applied(&self, data: &D, reverse: bool) -> bool;
    fn apply_into<W: io::Write>(
        &self,
        pd: &mut PatchableData<T, D>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool>;
    fn already_applied_into<W: io::Write>(
        &self,
        pd: &mut PatchableData<T, D>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool>;
}

pub trait ApplyChunksClean<'a, T, D, C>
where
    T: 'a + PartialEq + Clone,
    D: DataIfce<T> + WriteDataInto + Clone,
    C: ApplyChunkClean<T, D>,
{
    fn chunks<'b>(&'b self) -> impl Iterator<Item = &'b C>
    where
        C: 'b;

    fn apply_into<W: io::Write>(
        &self,
        patchable: &'a D,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool> {
        let mut pd = PatchableData::<T, D>::new(patchable);
        let mut iter = self.chunks();
        let mut chunk_num = 0;
        let mut success = true;
        while let Some(chunk) = iter.next() {
            chunk_num += 1; // for human consumption
            if chunk.will_apply(patchable, reverse) {
                chunk.apply_into(&mut pd, into, reverse)?;
                log::info!("Chunk #{chunk_num} applies cleanly.");
            } else if chunk.is_already_applied(patchable, reverse) {
                chunk.already_applied_into(&mut pd, into, reverse)?;
                log::warn!("Chunk #{chunk_num} already applied");
            } else {
                success = false;
                log::error!("Chunk #{chunk_num} could NOT be applied!");
            }
        }
        success &= pd.write_remainder(into)?;
        Ok(success)
    }

    fn already_applied(&self, patchable: &D, reverse: bool) -> bool {
        let mut chunk_num = 0;
        let mut iter = self.chunks().peekable();
        while let Some(chunk) = iter.next() {
            chunk_num += 1; // for human consumption
            if chunk.is_already_applied(patchable, reverse) {
                log::info!("Chunk #{chunk_num} already applied")
            } else {
                log::error!("Chunk #{chunk_num} NOT already applied!");
                return false;
            }
        }
        true
    }
}

pub trait ApplyChunkFuzzy<T, D>
where
    T: PartialEq + Clone,
    D: DataIfce<T> + WriteDataInto + Clone,
{
    fn will_apply(&self, patchable: &D, offset: isize, reverse: bool) -> Option<WillApply>;
    fn apply_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut PatchableData<T, D>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()>;
    fn will_apply_nearby(
        &self,
        pd: &PatchableData<T, D>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)>;
    fn is_already_applied(&self, patchable: &D, offset: isize, reverse: bool) -> Option<WillApply>;
    fn is_already_applied_nearby(
        &self,
        pd: &PatchableData<T, D>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)>;
    fn already_applied_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut PatchableData<T, D>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()>;
    fn write_failure_data_into<W: io::Write>(&self, into: &mut W, reverse: bool) -> io::Result<()>;
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum WillApply {
    Cleanly,
    WithReductions((u8, u8)),
}

#[derive(Debug, Default)]
pub struct Statistics {
    pub clean: usize,
    pub fuzzy: usize,
    pub already_applied: usize,
    pub already_applied_fuzzy: usize,
    pub failed: usize,
}

pub trait ApplyChunksFuzzy<T, D, C>
where
    T: PartialEq + Clone,
    D: DataIfce<T> + WriteDataInto + Clone,
    C: ApplyChunkFuzzy<T, D>,
{
    fn chunks<'b>(&'b self) -> impl Iterator<Item = &'b C>
    where
        C: 'b;

    fn apply_into<W: io::Write>(
        &self,
        patchable: &D,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<Statistics> {
        let mut pd = PatchableData::<T, D>::new(patchable);
        let mut stats = Statistics::default();
        let mut iter = self.chunks().peekable();
        let mut chunk_num = 0;
        let mut offset: isize = 0;
        while let Some(chunk) = iter.next() {
            chunk_num += 1; // for human consumption
            if let Some(will_apply) = chunk.will_apply(patchable, offset, reverse) {
                match will_apply {
                    WillApply::Cleanly => {
                        chunk.apply_into(into, &mut pd, offset, None, reverse)?;
                        stats.clean += 1;
                        log::info!("Chunk #{chunk_num} applies cleanly.");
                    }
                    WillApply::WithReductions(reductions) => {
                        chunk.apply_into(into, &mut pd, offset, Some(reductions), reverse)?;
                        stats.fuzzy += 1;
                        log::warn!("Chunk #{chunk_num} applies with {reductions:?} reductions.");
                    }
                }
            } else if let Some((offset_adj, will_apply)) =
                chunk.will_apply_nearby(&pd, iter.peek().copied(), offset, reverse)
            {
                offset += offset_adj;
                match will_apply {
                    WillApply::Cleanly => {
                        chunk.apply_into(into, &mut pd, offset, None, reverse)?;
                        stats.fuzzy += 1;
                        log::warn!("Chunk #{chunk_num} applies with offset {offset_adj}.");
                    }
                    WillApply::WithReductions(reductions) => {
                        chunk.apply_into(into, &mut pd, offset, Some(reductions), reverse)?;
                        stats.fuzzy += 1;
                        log::warn!("Chunk #{chunk_num} applies with {reductions:?} reductions and offset {offset_adj}.");
                    }
                }
            } else if let Some(appplied) = chunk.is_already_applied(patchable, offset, reverse) {
                match appplied {
                    WillApply::Cleanly => {
                        chunk.already_applied_into(into, &mut pd, offset, None, reverse)?;
                        stats.already_applied += 1;
                        log::warn!("Chunk #{chunk_num} already applied")
                    }
                    WillApply::WithReductions(reductions) => {
                        chunk.already_applied_into(
                            into,
                            &mut pd,
                            offset,
                            Some(reductions),
                            reverse,
                        )?;
                        stats.already_applied_fuzzy += 1;
                        log::warn!(
                            "Chunk #{chunk_num} already applied with {reductions:?} reductions."
                        );
                    }
                }
            } else if let Some((offset_adj, applied)) =
                chunk.is_already_applied_nearby(&pd, iter.peek().copied(), offset, reverse)
            {
                offset += offset_adj;
                match applied {
                    WillApply::Cleanly => {
                        chunk.already_applied_into(into, &mut pd, offset, None, reverse)?;
                        stats.already_applied_fuzzy += 1;
                        log::warn!("Chunk #{chunk_num} already applied with offset {offset_adj}")
                    }
                    WillApply::WithReductions(reductions) => {
                        chunk.already_applied_into(
                            into,
                            &mut pd,
                            offset,
                            Some(reductions),
                            reverse,
                        )?;
                        stats.already_applied_fuzzy += 1;
                        log::warn!("Chunk #{chunk_num} already applied with {reductions:?} reductions and offset {offset_adj}.")
                    }
                }
            } else {
                stats.failed += 1;
                chunk.write_failure_data_into(into, reverse)?;
                log::error!("Chunk #{chunk_num} could NOT be applied!");
            }
        }
        let ok = pd.write_remainder(into)?;
        debug_assert!(ok);
        Ok(stats)
    }

    fn is_already_applied(&self, patchable: &D, reverse: bool) -> bool {
        let pd = PatchableData::<T, D>::new(patchable);
        let mut iter = self.chunks().peekable();
        let mut chunk_num = 0;
        let mut offset: isize = 0;
        while let Some(chunk) = iter.next() {
            chunk_num += 1; // for human consumption
            if let Some(applied) = chunk.is_already_applied(patchable, offset, reverse) {
                match applied {
                    WillApply::Cleanly => {
                        log::info!("Chunk #{chunk_num} already applied")
                    }
                    WillApply::WithReductions(reductions) => {
                        log::warn!(
                            "Chunk #{chunk_num} already applied with {reductions:?} reductions."
                        );
                    }
                }
            } else if let Some((offset_adj, applied)) =
                chunk.is_already_applied_nearby(&pd, iter.peek().copied(), offset, reverse)
            {
                offset += offset_adj;
                match applied {
                    WillApply::Cleanly => {
                        log::warn!("Chunk #{chunk_num} already applied with offset {offset_adj}")
                    }
                    WillApply::WithReductions(reductions) => {
                        log::warn!("Chunk #{chunk_num} already applied with {reductions:?} reductions and offset {offset_adj}.")
                    }
                }
            } else {
                log::error!("Chunk #{chunk_num} NOT already applied!");
                return false;
            }
        }
        true
    }
}

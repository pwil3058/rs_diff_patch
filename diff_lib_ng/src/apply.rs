// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::data::{DataIfce, WriteDataInto};
use crate::range::Range;
use crate::snippet::{SnippetIfec, SnippetWrite};
use std::cmp::Ordering;
use std::io;
use std::marker::PhantomData;

use log;

#[derive(Debug, Clone)]
pub struct PatchableData<'a, T, D>
where
    T: PartialEq,
    D: DataIfce<T> + WriteDataInto,
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
    fn range_from(&self, from: usize) -> Range;
    fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool;
    fn incr_consumed(&mut self, increment: usize);
    fn write_upto_into<W: io::Write>(&mut self, upto: usize, writer: &mut W) -> io::Result<bool>;
    fn write_remainder<W: io::Write>(&mut self, writer: &mut W) -> io::Result<bool>;
}

impl<'a, T: PartialEq, D: DataIfce<T> + WriteDataInto> PatchableDataIfce<'a, T, D>
    for PatchableData<'a, T, D>
{
    fn new(data: &'a D) -> Self {
        Self {
            data,
            consumed: 0,
            phantom_data: PhantomData,
        }
    }

    fn range_from(&self, from: usize) -> Range {
        Range(from, self.data.len())
    }

    #[inline]
    fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool {
        self.data.has_subsequence_at(subsequence, at)
    }

    fn incr_consumed(&mut self, increment: usize) {
        self.consumed += increment
    }

    fn write_upto_into<W: io::Write>(&mut self, upto: usize, writer: &mut W) -> io::Result<bool> {
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

pub trait AppliableChunk<
    'a,
    T: PartialEq,
    D: DataIfce<T> + WriteDataInto,
    S: SnippetIfec<T> + SnippetWrite,
>
{
    // fn before(&self, reverse: bool) -> &S;
    // fn after(&self, reverse: bool) -> &S;
    fn before_start(&self, fuzz: Option<(isize, (usize, usize))>, reverse: bool) -> usize;
    fn before_items(&self, reductions: Option<(usize, usize)>, reverse: bool) -> &[T];
    fn before_length(&self, reductions: Option<(usize, usize)>, reverse: bool) -> usize;
    fn after_write_into<W: io::Write>(
        &self,
        writer: &mut W,
        reductions: Option<(u8, u8)>,
    ) -> io::Result<()>;

    fn applies_cleanly(&self, pd: &PatchableData<'a, T, D>, reverse: bool) -> bool {
        pd.has_subsequence_at(
            &self.before_items(None, reverse),
            self.before_start(None, reverse),
        )
    }

    fn apply_into_cleanly<W: io::Write>(
        &self,
        pd: &mut PatchableData<'a, T, D>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool> {
        let start = self.before_start(None, reverse);
        if pd.write_upto_into(start, into)? {
            let _ = self.after_write_into(into, None);
            pd.incr_consumed(self.before_length(None, reverse));
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub trait ApplyChunkClean<'a, T, D>
where
    T: PartialEq,
    D: DataIfce<T> + WriteDataInto,
{
    fn applies(&self, data: &D, reverse: bool) -> bool;
    fn already_applied(&self, data: &D, reverse: bool) -> bool;
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
    T: 'a + PartialEq,
    D: DataIfce<T> + WriteDataInto,
    C: ApplyChunkClean<'a, T, D>,
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
            if chunk.applies(patchable, reverse) {
                chunk.apply_into(&mut pd, into, reverse)?;
                log::info!("Chunk #{chunk_num} applies cleanly.");
            } else if chunk.already_applied(patchable, reverse) {
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
            if chunk.already_applied(patchable, reverse) {
                log::info!("Chunk #{chunk_num} already applied")
            } else {
                log::error!("Chunk #{chunk_num} NOT already applied!");
                return false;
            }
        }
        true
    }
}

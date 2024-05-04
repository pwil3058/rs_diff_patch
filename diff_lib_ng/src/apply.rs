// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::data::{DataIfce, WriteDataInto};
use crate::range::Range;
use crate::snippet::{SnippetIfec, SnippetWrite};
use std::cmp::Ordering;
use std::io;
use std::marker::PhantomData;
use std::ops::Deref;

pub struct PatchableData<'a, T: PartialEq, D: DataIfce<T>> {
    data: &'a D,
    consumed: usize,
    phantom_data: PhantomData<&'a T>,
}

impl<'a, T: PartialEq, D: DataIfce<T>> Deref for PatchableData<'a, T, D> {
    type Target = D;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

pub trait PatchableDataIfce<'a, T: PartialEq, D: DataIfce<T>> {
    fn new(data: &'a D) -> Self;
    fn incr_consumed(&mut self, increment: usize);
    fn write_upto_into<W: io::Write>(&mut self, upto: usize, writer: &mut W) -> io::Result<bool>;
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

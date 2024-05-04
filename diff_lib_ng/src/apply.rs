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
    fn before(&self, reverse: bool) -> &S;
    fn after(&self, reverse: bool) -> &S;

    fn applies_cleanly(&self, pd: &PatchableData<'a, T, D>, reverse: bool) -> bool {
        let before = self.before(reverse);
        pd.has_subsequence_at(&before.items(), before.start())
    }

    fn apply_into_cleanly<W: io::Write>(
        &self,
        pd: &mut PatchableData<'a, T, D>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool> {
        let before = self.before(reverse);
        if pd.write_upto_into(before.start(), into)? {
            let after = self.after(reverse);
            let _ = after.write_into(into, None);
            pd.incr_consumed(before.len());
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

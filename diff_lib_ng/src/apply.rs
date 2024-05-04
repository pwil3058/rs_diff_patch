// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::data::{Data, WriteDataInto};
use crate::range::{Len, Range};
use crate::snippet::Snippet;
use std::cmp::Ordering;
use std::io;
use std::ops::Deref;

struct PatchableData<'a, T: PartialEq> {
    data: &'a Data<T>,
    consumed: usize,
}

impl<'a, T: PartialEq> Deref for PatchableData<'a, T> {
    type Target = Data<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T: PartialEq> PatchableData<'a, T> {
    pub fn new(data: &'a Data<T>) -> Self {
        Self { data, consumed: 0 }
    }

    pub fn incr_consumed(&mut self, increment: usize) {
        self.consumed += increment
    }
}

// pub trait WriteUptoInto {
//     fn write_upto_into<W: io::Write>(&mut self, upto: usize, writer: &mut W) -> io::Result<bool>;
// }
//
// impl<'a> PatchableData<'a, u8> {
//     pub fn write_upto_into<W: io::Write>(
//         &mut self,
//         upto: usize,
//         writer: &mut W,
//     ) -> io::Result<bool> {
//         if upto <= self.data.len() {
//             match self.consumed.cmp(&upto) {
//                 Ordering::Less => {
//                     let range = Range(self.consumed, upto);
//                     self.consumed = upto;
//                     self.data.write_into(writer, range)
//                 }
//                 Ordering::Equal => Ok(true),
//                 Ordering::Greater => Ok(false),
//             }
//         } else {
//             Ok(false)
//         }
//     }
// }
//
// // TODO: do this with a trait
// impl<'a> PatchableData<'a, String> {
//     pub fn write_upto_into<W: io::Write>(
//         &mut self,
//         upto: usize,
//         writer: &mut W,
//     ) -> io::Result<bool> {
//         if upto <= self.data.len() {
//             match self.consumed.cmp(&upto) {
//                 Ordering::Less => {
//                     let range = Range(self.consumed, upto);
//                     self.consumed = upto;
//                     self.data.write_into(writer, range)
//                 }
//                 Ordering::Equal => Ok(true),
//                 Ordering::Greater => Ok(false),
//             }
//         } else {
//             Ok(false)
//         }
//     }
// }

pub trait AppliableChunk<'a, T: PartialEq> {
    fn before(&self, reverse: bool) -> &Snippet<T>;
    fn after(&self, reverse: bool) -> &Snippet<T>;

    fn applies_cleanly(&self, pd: &PatchableData<'a, T>, reverse: bool) -> bool {
        let before = self.before(reverse);
        pd.has_subsequence_at(&before.items, before.start)
    }

    // fn apply_into_cleanly<W: io::Write>(
    //     &self,
    //     pd: &PatchableData<'a, T>,
    //     into: &mut W,
    //     reverse: bool,
    // ) -> io::Result<bool>
    // where
    //     PatchableData<'a, T>: WriteDataInto,
    // {
    //     let before = self.before(reverse);
    //     if pd.write_upto_into(before.start, into)? {
    //         let after = self.after(reverse);
    //         after.write_into(into, None);
    //         pd.incr_consumed(before.length(None));
    //         Ok(true)
    //     } else {
    //         Ok(false)
    //     }
    // }
}

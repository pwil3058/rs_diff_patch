// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::io;

use log;

use crate::sequence::{ConsumableSeq, ConsumableSeqIfce, Seq};

pub trait ApplyClumpClean {
    fn will_apply(&self, se: &Seq<u8>, reverse: bool) -> bool;
    fn is_already_applied(&self, se: &Seq<u8>, reverse: bool) -> bool;
    fn apply_into<W: io::Write>(
        &self,
        pd: &mut ConsumableSeq<u8>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<()>;
    fn already_applied_into<W: io::Write>(
        &self,
        pd: &mut ConsumableSeq<u8>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<()>;
}

pub trait ApplyClumpsClean<'a, C>
where
    C: ApplyClumpClean,
{
    fn clumps<'b>(&'b self) -> impl Iterator<Item = &'b C>
    where
        C: 'b;

    fn apply_into<W: io::Write>(
        &self,
        patchable: &'a Seq<u8>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<()> {
        let mut pd = ConsumableSeq::<u8>::new(patchable);
        let mut iter = self.clumps();
        let mut clump_num = 0;
        while let Some(clump) = iter.next() {
            clump_num += 1; // for human consumption
            if clump.will_apply(patchable, reverse) {
                clump.apply_into(&mut pd, into, reverse)?;
                log::info!("Clump #{clump_num} applies cleanly.");
            } else if clump.is_already_applied(patchable, reverse) {
                clump.already_applied_into(&mut pd, into, reverse)?;
                log::warn!("Clump #{clump_num} already applied");
            } else {
                log::error!("Clump #{clump_num} could NOT be applied!");
            }
        }
        pd.write_remainder(into)
    }

    fn already_applied(&self, patchable: &Seq<u8>, reverse: bool) -> bool {
        let mut clump_num = 0;
        let mut iter = self.clumps().peekable();
        while let Some(clump) = iter.next() {
            clump_num += 1; // for human consumption
            if clump.is_already_applied(patchable, reverse) {
                log::info!("Clump #{clump_num} already applied")
            } else {
                log::error!("Clump #{clump_num} NOT already applied!");
                return false;
            }
        }
        true
    }
}

// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::data::{ConsumableData, ConsumableDataIfce, Data};
use std::io;

use log;

pub trait ApplyChunkClean {
    fn will_apply(&self, data: &Data<u8>, reverse: bool) -> bool;
    fn is_already_applied(&self, data: &Data<u8>, reverse: bool) -> bool;
    fn apply_into<W: io::Write>(
        &self,
        pd: &mut ConsumableData<u8, Data<u8>>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool>;
    fn already_applied_into<W: io::Write>(
        &self,
        pd: &mut ConsumableData<u8, Data<u8>>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool>;
}

pub trait ApplyChunksClean<'a, C>
where
    C: ApplyChunkClean,
{
    fn chunks<'b>(&'b self) -> impl Iterator<Item = &'b C>
    where
        C: 'b;

    fn apply_into<W: io::Write>(
        &self,
        patchable: &'a Data<u8>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<bool> {
        let mut pd = ConsumableData::<u8, Data<u8>>::new(patchable);
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

    fn already_applied(&self, patchable: &Data<u8>, reverse: bool) -> bool {
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

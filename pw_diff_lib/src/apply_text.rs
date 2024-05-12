// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::data::{ConsumableData, ConsumableDataIfce, DataIfce, WriteDataInto};
use crate::range::Range;
use std::io;

use log;

pub trait ApplyChunkFuzzy<T, D>
where
    T: PartialEq + Clone,
    D: DataIfce<T> + WriteDataInto + Clone,
{
    fn will_apply(&self, patchable: &D, offset: isize, reverse: bool) -> Option<WillApply>;
    fn apply_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut ConsumableData<T, D>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()>;
    fn will_apply_nearby(
        &self,
        pd: &ConsumableData<T, D>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)>;
    fn is_already_applied(&self, patchable: &D, offset: isize, reverse: bool) -> Option<WillApply>;
    fn is_already_applied_nearby(
        &self,
        pd: &ConsumableData<T, D>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)>;
    fn already_applied_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut ConsumableData<T, D>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()>;
    fn write_failure_data_into<W: io::Write>(&self, into: &mut W, reverse: bool) -> io::Result<()>;
}

pub trait ApplyChunkFuzzyBasics<T, D>
where
    T: PartialEq + Clone,
    D: DataIfce<T> + WriteDataInto + Clone,
{
    fn context_lengths(&self) -> (u8, u8);
    fn before_start(&self, reverse: bool) -> usize;
    fn before_length(&self, reverse: bool) -> usize;
    fn before_items<'a>(
        &'a self,
        range: Option<Range>,
        reverse: bool,
    ) -> impl Iterator<Item = &'a T>
    where
        T: 'a;
    fn before_write_into<W: io::Write>(
        &self,
        into: &mut W,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()>;

    fn after_start(&self, reverse: bool) -> usize {
        self.before_start(!reverse)
    }

    fn after_length(&self, reverse: bool) -> usize {
        self.before_length(!reverse)
    }

    fn after_items<'a>(&'a self, range: Option<Range>, reverse: bool) -> impl Iterator<Item = &'a T>
    where
        T: 'a,
    {
        self.before_items(range, !reverse)
    }

    fn after_write_into<W: io::Write>(
        &self,
        into: &mut W,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        self.before_write_into(into, reductions, !reverse)
    }
}

pub trait ApplyChunkFuzzy2<T, D>: ApplyChunkFuzzyBasics<T, D>
where
    T: PartialEq + Clone,
    D: DataIfce<T> + WriteDataInto + Clone,
{
    fn before_adjusted_start(
        &self,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> isize {
        if let Some((start_redn, _)) = reductions {
            self.before_start(reverse) as isize + offset + start_redn as isize
        } else {
            self.before_start(reverse) as isize + offset
        }
    }
    fn after_adjusted_start(
        &self,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> isize {
        self.before_adjusted_start(offset, reductions, !reverse)
    }

    fn before_adjusted_length(&self, reductions: Option<(u8, u8)>, reverse: bool) -> usize {
        if let Some((start_redn, end_redn)) = reductions {
            self.before_length(reverse) - start_redn as usize - end_redn as usize
        } else {
            self.before_length(reverse)
        }
    }

    fn after_adjusted_length(&self, reductions: Option<(u8, u8)>, reverse: bool) -> usize {
        self.before_adjusted_length(reductions, !reverse)
    }

    fn before_is_subsequence_in_at(
        &self,
        patchable: &D,
        at: usize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> bool {
        let length = self.before_adjusted_length(reductions, reverse);
        let end = at + length;
        if end > patchable.len() {
            false
        } else if let Some(reductions) = reductions {
            let my_range = Range(reductions.0 as usize, length - reductions.1 as usize);
            let other_range = Range(at, end);
            self.before_items(Some(my_range), reverse)
                .zip(patchable.subsequence(other_range))
                .all(|(l, r)| l == r)
        } else {
            let other_range = Range(at, end);
            self.before_items(None, reverse)
                .zip(patchable.subsequence(other_range))
                .all(|(l, r)| l == r)
        }
    }

    fn will_apply(&self, patchable: &D, offset: isize, reverse: bool) -> Option<WillApply> {
        let start = self.before_adjusted_start(offset, None, reverse);
        if !start.is_negative()
            && self.before_is_subsequence_in_at(patchable, start as usize, None, reverse)
        {
            Some(WillApply::Cleanly)
        } else {
            let (start_context_len, end_context_len) = self.context_lengths();
            let max_reduction = start_context_len.max(end_context_len);
            for redn in 1..max_reduction {
                let start_redn = redn.min(start_context_len);
                let end_redn = redn.min(end_context_len);
                let adj_start = start + start_redn as isize;
                if !adj_start.is_negative()
                    && self.before_is_subsequence_in_at(
                        patchable,
                        adj_start as usize,
                        Some((start_redn, end_redn)),
                        reverse,
                    )
                {
                    return Some(WillApply::WithReductions((start_redn, end_redn)));
                }
            }
            None
        }
    }

    fn apply_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut ConsumableData<T, D>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        let end = self.before_adjusted_start(offset, reductions, reverse) as usize;
        pd.write_into_upto(into, end)?;
        self.after_write_into(into, reductions, reverse)?;
        pd.advance_consumed_by(self.before_adjusted_length(reductions, reverse));
        Ok(())
    }

    fn will_apply_nearby(
        &self,
        pd: &ConsumableData<T, D>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)> {
        let not_after = if let Some(next_chunk) = next_chunk {
            next_chunk.before_adjusted_start(offset, None, reverse) as usize
                - self.before_adjusted_length(None, reverse)
        } else {
            pd.data().len() - self.before_adjusted_length(None, reverse)
        };
        let mut backward_done = false;
        let mut forward_done = false;
        for i in 1isize.. {
            if !backward_done {
                let adjusted_offset = offset - i;
                if self.before_adjusted_start(adjusted_offset, None, reverse)
                    < pd.consumed() as isize
                {
                    backward_done = true;
                } else {
                    if let Some(will_apply) = self.will_apply(pd.data(), adjusted_offset, reverse) {
                        return Some((-i, will_apply));
                    }
                }
            }
            if !forward_done {
                let adjusted_offset = offset + i;
                if self.before_adjusted_start(adjusted_offset, None, reverse) < not_after as isize {
                    if let Some(will_apply) = self.will_apply(pd.data(), adjusted_offset, reverse) {
                        return Some((i, will_apply));
                    }
                } else {
                    forward_done = true
                }
            }
            if forward_done && backward_done {
                break;
            }
        }
        None
    }

    fn is_already_applied(&self, patchable: &D, offset: isize, reverse: bool) -> Option<WillApply> {
        self.will_apply(patchable, offset, !reverse)
    }

    fn is_already_applied_nearby(
        &self,
        pd: &ConsumableData<T, D>,
        next_chunk: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)> {
        self.will_apply_nearby(pd, next_chunk, offset, !reverse)
    }

    fn already_applied_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut ConsumableData<T, D>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        let end = self.after_adjusted_start(offset, reductions, reverse) as usize
            + self.after_adjusted_length(reductions, reverse);
        let ok = pd.write_into_upto(into, end)?;
        debug_assert!(ok);
        Ok(())
    }

    fn write_failure_data_into<W: io::Write>(&self, into: &mut W, reverse: bool) -> io::Result<()> {
        into.write_all(b"<<<<<<<\n")?;
        self.before_write_into(into, None, reverse)?;
        into.write_all(b"=======\n")?;
        self.after_write_into(into, None, reverse)?;
        into.write_all(b">>>>>>>\n")
    }
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
        let mut pd = ConsumableData::<T, D>::new(patchable);
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
        let pd = ConsumableData::<T, D>::new(patchable);
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

#[cfg(test)]
mod apply_tests;

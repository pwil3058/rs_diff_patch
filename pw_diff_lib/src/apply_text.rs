// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::io;

use log;

use crate::changes::ChangeBasics;
use crate::range::{Len, Range};
use crate::sequence::{ConsumableSeq, ConsumableSeqIfce, Seq};

pub trait TextClumpBasics: ChangeBasics {
    fn context_lengths(&self) -> (u8, u8);
    fn before_lines(&self, range: Option<Range>, reverse: bool) -> impl Iterator<Item = &String>;
    fn after_lines(&self, range: Option<Range>, reverse: bool) -> impl Iterator<Item = &String> {
        self.before_lines(range, !reverse)
    }
}

pub trait ApplyClumpFuzzy: TextClumpBasics {
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
        patchable: &Seq<String>,
        at: usize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> bool {
        let my_range = self.my_before_range(reductions, reverse);
        let end = at + my_range.len();
        if end > patchable.len() {
            false
        } else {
            let other_range = Range(at, end);
            self.before_lines(Some(my_range), reverse)
                .zip(patchable.subsequence(other_range))
                .all(|(l, r)| l == r)
        }
    }

    fn before_write_into<W: io::Write>(
        &self,
        into: &mut W,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        if reductions.is_some() {
            let range = self.before_range(reductions, reverse);
            for line in self.before_lines(Some(range), reverse) {
                into.write_all(line.as_bytes())?;
            }
        } else {
            for line in self.before_lines(None, reverse) {
                into.write_all(line.as_bytes())?;
            }
        };
        Ok(())
    }

    fn after_write_into<W: io::Write>(
        &self,
        into: &mut W,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        self.before_write_into(into, reductions, !reverse)
    }

    fn will_apply(
        &self,
        patchable: &Seq<String>,
        offset: isize,
        reverse: bool,
    ) -> Option<WillApply> {
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
        pd: &mut ConsumableSeq<String>,
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
        pd: &ConsumableSeq<String>,
        next_clump: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)> {
        let not_after = if let Some(next_clump) = next_clump {
            next_clump.before_adjusted_start(offset, Some(self.context_lengths()), reverse) as usize
                - self.before_adjusted_length(Some(self.context_lengths()), reverse)
        } else {
            pd.data().len() - self.before_adjusted_length(Some(self.context_lengths()), reverse)
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

    fn is_already_applied(
        &self,
        patchable: &Seq<String>,
        offset: isize,
        reverse: bool,
    ) -> Option<WillApply> {
        self.will_apply(patchable, offset, !reverse)
    }

    fn is_already_applied_nearby(
        &self,
        pd: &ConsumableSeq<String>,
        next_clump: Option<&Self>,
        offset: isize,
        reverse: bool,
    ) -> Option<(isize, WillApply)> {
        self.will_apply_nearby(pd, next_clump, offset, !reverse)
    }

    fn already_applied_into<W: io::Write>(
        &self,
        into: &mut W,
        pd: &mut ConsumableSeq<String>,
        offset: isize,
        reductions: Option<(u8, u8)>,
        reverse: bool,
    ) -> io::Result<()> {
        let end = self.after_adjusted_start(offset, reductions, reverse) as usize
            + self.after_adjusted_length(reductions, reverse);
        pd.write_into_upto(into, end)
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

pub trait ApplyClumpsFuzzy<C>
where
    C: ApplyClumpFuzzy,
{
    fn clumps<'b>(&'b self) -> impl Iterator<Item = &'b C>
    where
        C: 'b;

    fn apply_into<W: io::Write>(
        &self,
        patchable: &Seq<String>,
        into: &mut W,
        reverse: bool,
    ) -> io::Result<Statistics> {
        let mut pd = ConsumableSeq::<String>::new(patchable);
        let mut stats = Statistics::default();
        let mut iter = self.clumps().peekable();
        let mut clump_num = 0;
        let mut offset: isize = 0;
        while let Some(clump) = iter.next() {
            clump_num += 1; // for human consumption
            if let Some(will_apply) = clump.will_apply(patchable, offset, reverse) {
                match will_apply {
                    WillApply::Cleanly => {
                        clump.apply_into(into, &mut pd, offset, None, reverse)?;
                        stats.clean += 1;
                        log::info!("Clump #{clump_num} applies cleanly.");
                    }
                    WillApply::WithReductions(reductions) => {
                        clump.apply_into(into, &mut pd, offset, Some(reductions), reverse)?;
                        stats.fuzzy += 1;
                        log::warn!("Clump #{clump_num} applies with {reductions:?} reductions.");
                    }
                }
            } else if let Some((offset_adj, will_apply)) =
                clump.will_apply_nearby(&pd, iter.peek().copied(), offset, reverse)
            {
                offset += offset_adj;
                match will_apply {
                    WillApply::Cleanly => {
                        clump.apply_into(into, &mut pd, offset, None, reverse)?;
                        stats.fuzzy += 1;
                        log::warn!("Clump #{clump_num} applies with offset {offset_adj}.");
                    }
                    WillApply::WithReductions(reductions) => {
                        clump.apply_into(into, &mut pd, offset, Some(reductions), reverse)?;
                        stats.fuzzy += 1;
                        log::warn!("Clump #{clump_num} applies with {reductions:?} reductions and offset {offset_adj}.");
                    }
                }
            } else if let Some(appplied) = clump.is_already_applied(patchable, offset, reverse) {
                match appplied {
                    WillApply::Cleanly => {
                        clump.already_applied_into(into, &mut pd, offset, None, reverse)?;
                        stats.already_applied += 1;
                        log::warn!("Clump #{clump_num} already applied")
                    }
                    WillApply::WithReductions(reductions) => {
                        clump.already_applied_into(
                            into,
                            &mut pd,
                            offset,
                            Some(reductions),
                            reverse,
                        )?;
                        stats.already_applied_fuzzy += 1;
                        log::warn!(
                            "Clump #{clump_num} already applied with {reductions:?} reductions."
                        );
                    }
                }
            } else if let Some((offset_adj, applied)) =
                clump.is_already_applied_nearby(&pd, iter.peek().copied(), offset, reverse)
            {
                offset += offset_adj;
                match applied {
                    WillApply::Cleanly => {
                        clump.already_applied_into(into, &mut pd, offset, None, reverse)?;
                        stats.already_applied_fuzzy += 1;
                        log::warn!("Clump #{clump_num} already applied with offset {offset_adj}")
                    }
                    WillApply::WithReductions(reductions) => {
                        clump.already_applied_into(
                            into,
                            &mut pd,
                            offset,
                            Some(reductions),
                            reverse,
                        )?;
                        stats.already_applied_fuzzy += 1;
                        log::warn!("Clump #{clump_num} already applied with {reductions:?} reductions and offset {offset_adj}.")
                    }
                }
            } else {
                stats.failed += 1;
                clump.write_failure_data_into(into, reverse)?;
                log::error!("Clump #{clump_num} could NOT be applied!");
            }
        }
        pd.write_remainder(into)?;
        Ok(stats)
    }

    fn is_already_applied(&self, patchable: &Seq<String>, reverse: bool) -> bool {
        let pd = ConsumableSeq::<String>::new(patchable);
        let mut iter = self.clumps().peekable();
        let mut clump_num = 0;
        let mut offset: isize = 0;
        while let Some(clump) = iter.next() {
            clump_num += 1; // for human consumption
            if let Some(applied) = clump.is_already_applied(patchable, offset, reverse) {
                match applied {
                    WillApply::Cleanly => {
                        log::info!("Clump #{clump_num} already applied")
                    }
                    WillApply::WithReductions(reductions) => {
                        log::warn!(
                            "Clump #{clump_num} already applied with {reductions:?} reductions."
                        );
                    }
                }
            } else if let Some((offset_adj, applied)) =
                clump.is_already_applied_nearby(&pd, iter.peek().copied(), offset, reverse)
            {
                offset += offset_adj;
                match applied {
                    WillApply::Cleanly => {
                        log::warn!("Clump #{clump_num} already applied with offset {offset_adj}")
                    }
                    WillApply::WithReductions(reductions) => {
                        log::warn!("Clump #{clump_num} already applied with {reductions:?} reductions and offset {offset_adj}.")
                    }
                }
            } else {
                log::error!("Clump #{clump_num} NOT already applied!");
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod apply_tests;

// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use longest_common_subsequence::{range::Range, sequence::Seq};

use std::cmp::Ordering;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};

pub trait ReadSequence: Sized {
    fn read<R: Read>(read: R) -> io::Result<Self>;
}

impl ReadSequence for Seq<String> {
    fn read<R: Read>(read: R) -> io::Result<Self> {
        let mut reader = BufReader::new(read);
        let mut lines = vec![];
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 {
                break;
            } else {
                lines.push(line)
            }
        }
        Ok(Self(lines.into_boxed_slice()))
    }
}

impl ReadSequence for Seq<u8> {
    fn read<R: Read>(read: R) -> io::Result<Self> {
        let mut reader = BufReader::new(read);
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes)?;
        Ok(Self(bytes.into_boxed_slice()))
    }
}

pub trait WriteDataInto {
    fn write_into<W: io::Write>(&self, into: &mut W, range: Range) -> io::Result<()>;
    fn write_into_all_from<W: io::Write>(&self, into: &mut W, from: usize) -> io::Result<()>;
}

impl WriteDataInto for Seq<u8> {
    fn write_into<W: Write>(&self, into: &mut W, range: Range) -> io::Result<()> {
        debug_assert!(range.is_valid_for_max_end(self.len()));
        into.write_all(&self.0[range.start()..range.end()])
    }

    fn write_into_all_from<W: io::Write>(&self, into: &mut W, from: usize) -> io::Result<()> {
        debug_assert!(from <= self.len());
        into.write_all(&self.0[from..])
    }
}

impl WriteDataInto for Seq<String> {
    fn write_into<W: Write>(&self, into: &mut W, range: Range) -> io::Result<()> {
        debug_assert!(range.is_valid_for_max_end(self.len()));
        for datum in self.0[range.start()..range.end()].iter() {
            into.write_all(datum.as_bytes())?;
        }
        Ok(())
    }

    fn write_into_all_from<W: io::Write>(&self, into: &mut W, from: usize) -> io::Result<()> {
        debug_assert!(from <= self.len());
        for datum in self.0[from..].iter() {
            into.write_all(datum.as_bytes())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConsumableSeq<'a, T>
where
    T: PartialEq + Clone,
{
    sequence: &'a Seq<T>,
    consumed: usize,
}

pub trait ConsumableSeqIfce<'a, T: PartialEq + Clone>
where
    Seq<T>: WriteDataInto,
{
    fn new(data: &'a Seq<T>) -> Self;
    fn data(&self) -> &Seq<T>;
    fn consumed(&self) -> usize;
    fn range_from(&self, from: usize) -> Range;
    fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool;
    fn advance_consumed_by(&mut self, increment: usize);
    fn write_into_upto<W: io::Write>(&mut self, writer: &mut W, upto: usize) -> io::Result<()>;
    fn write_remainder<W: io::Write>(&mut self, writer: &mut W) -> io::Result<()>;
}

impl<'a, T: PartialEq + Clone> ConsumableSeqIfce<'a, T> for ConsumableSeq<'a, T>
where
    Seq<T>: WriteDataInto,
{
    fn new(sequence: &'a Seq<T>) -> Self {
        Self {
            sequence,
            consumed: 0,
        }
    }

    #[inline]
    fn data(&self) -> &Seq<T> {
        self.sequence
    }

    #[inline]
    fn consumed(&self) -> usize {
        self.consumed
    }

    fn range_from(&self, from: usize) -> Range {
        Range(from, self.sequence.len())
    }

    #[inline]
    fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool {
        self.sequence.has_subsequence_at(subsequence, at)
    }

    fn advance_consumed_by(&mut self, increment: usize) {
        self.consumed += increment
    }

    fn write_into_upto<W: io::Write>(&mut self, writer: &mut W, upto: usize) -> io::Result<()> {
        debug_assert!(upto <= self.sequence.len());
        match self.consumed.cmp(&upto) {
            Ordering::Less => {
                let range = Range(self.consumed, upto);
                self.consumed = upto;
                self.sequence.write_into(writer, range)
            }
            Ordering::Equal => Ok(()),
            Ordering::Greater => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Attempt to write non existent data.",
            )),
        }
    }

    fn write_remainder<W: io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        let range = self.range_from(self.consumed);
        self.consumed = self.sequence.len();
        self.sequence.write_into(writer, range)
    }
}

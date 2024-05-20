// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::range::Range;
use crate::snippet::Snippet;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};
use std::ops::Deref;

#[derive(Debug, Default, PartialEq)]
pub struct Seq<T: PartialEq + Clone>(Box<[T]>);

impl<T: PartialEq + Clone> Deref for Seq<T> {
    type Target = Box<[T]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: PartialEq + Clone> Seq<T> {
    pub fn range_from(&self, from: usize) -> Range {
        Range(from, self.len())
    }

    pub fn subsequence(&self, range: Range) -> impl DoubleEndedIterator<Item = &T> {
        self.0[range.0..range.1].iter()
    }

    pub fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool {
        if at < self.len() && self.len() - at >= subsequence.len() {
            subsequence
                .iter()
                .zip(self.0[at..].iter())
                .all(|(b, a)| a == b)
        } else {
            false
        }
    }

    pub fn extract_snippet(&self, range: Range) -> Snippet<T> {
        let start = range.start();
        let items = self.0[range.0..range.1].to_vec().into_boxed_slice();
        Snippet { start, items }
    }
}

impl Seq<String> {
    pub fn read<R: Read>(read: R) -> io::Result<Self> {
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

impl Seq<u8> {
    pub fn read<R: Read>(read: R) -> io::Result<Self> {
        let mut reader = BufReader::new(read);
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes)?;
        Ok(Self(bytes.into_boxed_slice()))
    }
}

#[cfg(test)]
impl From<String> for Seq<String> {
    fn from(text: String) -> Self {
        Self(text.split_inclusive('\n').map(|s| s.to_string()).collect())
    }
}

#[cfg(test)]
impl From<&str> for Seq<String> {
    fn from(arg: &str) -> Self {
        Self::from(arg.to_string())
    }
}

#[cfg(test)]
impl From<Vec<u8>> for Seq<u8> {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes.into_boxed_slice())
    }
}

#[cfg(test)]
impl From<&[u8]> for Seq<u8> {
    fn from(bytes: &[u8]) -> Self {
        Self::from(bytes.to_vec())
    }
}

pub trait ContentItemIndices<T: PartialEq + Clone> {
    fn generate_from(sequence: &Seq<T>) -> Box<Self>
    where
        Self: Sized;
    fn indices(&self, item: &T) -> Option<&Vec<usize>>;
}

#[derive(Debug, Default)]
pub struct StringItemIndices(HashMap<String, Vec<usize>>);

impl ContentItemIndices<String> for StringItemIndices {
    fn generate_from(sequence: &Seq<String>) -> Box<Self> {
        let mut map = HashMap::<String, Vec<usize>>::new();
        for (index, line) in sequence.iter().enumerate() {
            if let Some(vec) = map.get_mut(line) {
                vec.push(index)
            } else {
                map.insert(line.to_string(), vec![index]);
            }
        }

        Box::new(Self(map))
    }

    fn indices(&self, item: &String) -> Option<&Vec<usize>> {
        self.0.get(item)
    }
}

#[derive(Debug)]
pub struct ByteItemIndices(pub [Vec<usize>; 256]);

impl ContentItemIndices<u8> for ByteItemIndices {
    fn generate_from(sequence: &Seq<u8>) -> Box<Self> {
        const ARRAY_REPEAT_VALUE: Vec<usize> = Vec::<usize>::new();
        let mut indices = [ARRAY_REPEAT_VALUE; 256];
        for (index, byte) in sequence.iter().enumerate() {
            indices[*byte as usize].push(index);
        }
        Box::new(Self(indices))
    }

    fn indices(&self, item: &u8) -> Option<&Vec<usize>> {
        let result = &self.0[*item as usize];
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
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
            Ordering::Greater => Ok(()),
        }
    }

    fn write_remainder<W: io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        let range = self.range_from(self.consumed);
        self.consumed = self.sequence.len();
        self.sequence.write_into(writer, range)
    }
}

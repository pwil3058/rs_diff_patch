// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::borrow::Borrow;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};
use std::ops::Deref;

use crate::range::Range;

/// A sequence of items of type T
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Seq<T: PartialEq + Clone>(pub Box<[T]>);

impl<T: PartialEq + Clone> Deref for Seq<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: PartialEq + Clone> AsRef<[T]> for Seq<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        &self.0
    }
}

impl<T: PartialEq + Clone> Borrow<[T]> for Seq<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        &self.0
    }
}

impl<T: PartialEq + Clone> Seq<T> {
    /// Returns the `Range` from the index `from` to the end of the sequence
    #[inline]
    pub fn range_from(&self, from: usize) -> Range {
        Range(from, self.len())
    }

    /// Returns an iterator over the items in `range`
    #[inline]
    pub fn subsequence(&self, range: Range) -> impl DoubleEndedIterator<Item = &T> {
        self.0[range.0..range.1].iter()
    }

    /// Returns `true` if the given subsequence exists at the given index,
    #[inline]
    pub fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool {
        if let Some(end) = at.checked_add(subsequence.len()) {
            end <= self.len() && self.0[at..end] == *subsequence
        } else {
            false // Arithmetic overflowed usize capacity
        }
    }
}

impl<T: PartialEq + Clone> From<&[T]> for Seq<T> {
    #[inline]
    fn from(slice: &[T]) -> Self {
        Seq(Box::from(slice))
    }
}

impl<T: PartialEq + Clone> From<Vec<T>> for Seq<T> {
    #[inline]
    fn from(vec: Vec<T>) -> Self {
        Seq(vec.into_boxed_slice())
    }
}

impl<T: PartialEq + Clone> FromIterator<T> for Seq<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Seq(iter.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }
}

impl<'a, T: PartialEq + Clone> FromIterator<&'a T> for Seq<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a T>>(iter: I) -> Self {
        Seq(iter
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
            .into_boxed_slice())
    }
}

pub trait SequenceIO: Sized {
    fn read_from<R: Read>(read: R) -> io::Result<Self>;
    fn write_into<W: io::Write>(&self, into: &mut W, range: Range) -> io::Result<()>;
    fn write_into_all_from<W: io::Write>(&self, into: &mut W, from: usize) -> io::Result<()>;
}

/// Sequence of text lines

impl SequenceIO for Seq<String> {
    fn read_from<R: Read>(read: R) -> io::Result<Self> {
        let mut reader = BufReader::new(read);
        let mut lines = vec![];

        let mut buf = String::new();
        while reader.read_line(&mut buf)? > 0 {
            lines.push(buf.clone());
            buf.clear(); // Keeps internal capacity allocation active for reuse
        }

        Ok(Self(lines.into_boxed_slice()))
    }

    fn write_into<W: Write>(&self, into: &mut W, range: Range) -> io::Result<()> {
        debug_assert!(range.is_valid_for_max_end(self.len()));
        for datum in &self.0[range.start()..range.end()] {
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

/// A sequence of bytes

impl SequenceIO for Seq<u8> {
    fn read_from<R: Read>(read: R) -> io::Result<Self> {
        let mut reader = BufReader::new(read);
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes)?;
        Ok(Self(bytes.into_boxed_slice()))
    }

    fn write_into<W: Write>(&self, into: &mut W, range: Range) -> io::Result<()> {
        debug_assert!(range.is_valid_for_max_end(self.len()));
        into.write_all(&self.0[range.start()..range.end()])
    }

    fn write_into_all_from<W: io::Write>(&self, into: &mut W, from: usize) -> io::Result<()> {
        debug_assert!(from <= self.len());
        into.write_all(&self.0[from..])
    }
}

// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::range::{Len, Range};
use crate::snippet::Snippet;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io;
use std::io::{BufRead, BufReader, Read, Write};
use std::marker::PhantomData;

pub trait ContentIndices<T> {
    fn indices(&self, key: &T) -> Option<&Vec<usize>>;
}

#[derive(Debug, Default)]
pub struct LineIndices(HashMap<String, Vec<usize>>);

impl ContentIndices<String> for LineIndices {
    fn indices(&self, key: &String) -> Option<&Vec<usize>> {
        self.0.get(key)
    }
}

#[derive(Debug)]
pub struct ByteIndices(pub [Vec<usize>; 256]);

impl ContentIndices<u8> for ByteIndices {
    fn indices(&self, key: &u8) -> Option<&Vec<usize>> {
        let result = &self.0[*key as usize];
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

pub trait GenerateContentIndices<T> {
    fn generate_content_indices(&self) -> impl ContentIndices<T>;
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Data<T: PartialEq>(Box<[T]>);

impl From<String> for Data<String> {
    fn from(text: String) -> Self {
        Self(text.split_inclusive('\n').map(|s| s.to_string()).collect())
    }
}

impl From<&str> for Data<String> {
    fn from(arg: &str) -> Self {
        Self::from(arg.to_string())
    }
}

impl From<Vec<u8>> for Data<u8> {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes.into_boxed_slice())
    }
}

impl<T: PartialEq> Len for Data<T> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl GenerateContentIndices<String> for Data<String> {
    /// Generate the content to index mechanism for this `Data`
    ///
    /// Example:
    /// ```
    /// use pw_diff_lib::data::*;
    /// let data = Data::<String>::from("A\nB\nC\nD\nA\nB\nC\nD\n");
    /// let indices = data.generate_content_indices();
    /// assert_eq!(indices.indices(&"A\n".to_string()),Some( &vec![0usize,4]));
    /// assert_eq!(indices.indices(&"C\n".to_string()),Some( &vec![2usize,6]));
    /// assert_eq!(indices.indices(&"E\n".to_string()),None);
    /// ```
    #[allow(refining_impl_trait)]
    fn generate_content_indices(&self) -> LineIndices {
        let mut map = HashMap::<String, Vec<usize>>::new();
        for (index, line) in self.0.iter().enumerate() {
            if let Some(vec) = map.get_mut(line) {
                vec.push(index)
            } else {
                map.insert(line.to_string(), vec![index]);
            }
        }

        LineIndices(map)
    }
}

impl GenerateContentIndices<u8> for Data<u8> {
    /// Generate the content to index mechanism for this `Data`
    ///
    /// Example:
    /// ```
    /// use pw_diff_lib::data::*;
    /// let data = Data::<u8>::from(vec![0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
    /// let indices = data.generate_content_indices();
    /// assert_eq!(indices.indices(&0u8),Some( &vec![0usize,17]));
    /// assert_eq!(indices.indices(&16u8),Some( &vec![16usize,33]));
    /// assert_eq!(indices.indices(&17u8),None);
    /// ```
    #[allow(refining_impl_trait)]
    fn generate_content_indices(&self) -> ByteIndices {
        const ARRAY_REPEAT_VALUE: Vec<usize> = Vec::<usize>::new();
        let mut indices = [ARRAY_REPEAT_VALUE; 256];
        for (index, byte) in self.0.iter().enumerate() {
            indices[*byte as usize].push(index);
        }
        ByteIndices(indices)
    }
}

pub trait WriteDataInto {
    fn write_into<W: io::Write>(&self, into: &mut W, range: Range) -> io::Result<bool>;
    fn write_into_all_from<W: io::Write>(&self, into: &mut W, from: usize) -> io::Result<()>;
}

impl WriteDataInto for Data<u8> {
    fn write_into<W: Write>(&self, into: &mut W, range: Range) -> io::Result<bool> {
        if range.end() > self.len() || range.start() > self.len() {
            Ok(false)
        } else {
            into.write_all(&self.0[range.start()..range.end()])?;
            Ok(true)
        }
    }

    fn write_into_all_from<W: io::Write>(&self, into: &mut W, from: usize) -> io::Result<()> {
        if from < self.len() {
            into.write_all(&self.0[from..])
        } else {
            Ok(())
        }
    }
}

impl WriteDataInto for Data<String> {
    fn write_into<W: Write>(&self, into: &mut W, range: Range) -> io::Result<bool> {
        if range.end() > self.len() || range.start() > self.len() {
            Ok(false)
        } else {
            for datum in self.0[range.start()..range.end()].iter() {
                into.write_all(datum.as_bytes())?;
            }
            Ok(true)
        }
    }

    fn write_into_all_from<W: io::Write>(&self, into: &mut W, from: usize) -> io::Result<()> {
        if from < self.len() {
            for datum in self.0[from..].iter() {
                into.write_all(datum.as_bytes())?;
            }
            Ok(())
        } else {
            Ok(())
        }
    }
}

pub trait ExtractSnippet<T> {
    fn extract_snippet(&self, range: Range) -> Snippet<T>;
}

impl ExtractSnippet<u8> for Data<u8> {
    fn extract_snippet(&self, range: Range) -> Snippet<u8> {
        let start = range.start();
        let items = self.0[range.0..range.1].to_vec().into_boxed_slice();
        Snippet { start, items }
    }
}

impl ExtractSnippet<String> for Data<String> {
    fn extract_snippet(&self, range: Range) -> Snippet<String> {
        let start = range.start();
        let items = self.0[range.0..range.1]
            .iter()
            .map(|s| s.to_string())
            .collect();
        Snippet { start, items }
    }
}

pub trait DataIfce<T: PartialEq>: Len {
    fn data(&self) -> &Box<[T]>;

    fn range_from(&self, from: usize) -> Range {
        Range(from, self.len())
    }

    fn subsequence<'a>(&'a self, range: Range) -> impl DoubleEndedIterator<Item = &'a T>
    where
        T: 'a,
    {
        self.data()[range.0..range.1].iter()
    }

    fn subsequence_from<'a>(&'a self, from: usize) -> impl DoubleEndedIterator<Item = &'a T>
    where
        T: 'a,
    {
        self.data()[from..].iter()
    }

    fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool {
        if at < self.len() && self.len() - at >= subsequence.len() {
            subsequence
                .iter()
                .zip(self.data()[at..].iter())
                .all(|(b, a)| a == b)
        } else {
            false
        }
    }
}

impl<T: PartialEq> DataIfce<T> for Data<T> {
    fn data(&self) -> &Box<[T]> {
        &self.0
    }
}

impl Data<String> {
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

impl Data<u8> {
    pub fn read<R: Read>(read: R) -> io::Result<Self> {
        let mut reader = BufReader::new(read);
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes)?;
        Ok(Self(bytes.into_boxed_slice()))
    }
}

#[derive(Debug, Clone)]
pub struct ConsumableData<'a, T, D>
where
    T: PartialEq + Clone,
    D: DataIfce<T> + WriteDataInto + Clone,
{
    data: &'a D,
    consumed: usize,
    phantom_data: PhantomData<&'a T>,
}

pub trait ConsumableDataIfce<'a, T: PartialEq, D: DataIfce<T>>
where
    D: WriteDataInto,
{
    fn new(data: &'a D) -> Self;
    fn data(&self) -> &D;
    fn consumed(&self) -> usize;
    fn range_from(&self, from: usize) -> Range;
    fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool;
    fn advance_consumed_by(&mut self, increment: usize);
    fn write_into_upto<W: io::Write>(&mut self, writer: &mut W, upto: usize) -> io::Result<bool>;
    fn write_remainder<W: io::Write>(&mut self, writer: &mut W) -> io::Result<bool>;
}

impl<'a, T: PartialEq + Clone, D: DataIfce<T> + WriteDataInto + Clone> ConsumableDataIfce<'a, T, D>
    for ConsumableData<'a, T, D>
{
    fn new(data: &'a D) -> Self {
        Self {
            data,
            consumed: 0,
            phantom_data: PhantomData,
        }
    }

    #[inline]
    fn data(&self) -> &D {
        self.data
    }

    #[inline]
    fn consumed(&self) -> usize {
        self.consumed
    }

    fn range_from(&self, from: usize) -> Range {
        Range(from, self.data.len())
    }

    #[inline]
    fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool {
        self.data.has_subsequence_at(subsequence, at)
    }

    fn advance_consumed_by(&mut self, increment: usize) {
        self.consumed += increment
    }

    fn write_into_upto<W: io::Write>(&mut self, writer: &mut W, upto: usize) -> io::Result<bool> {
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
    fn write_remainder<W: io::Write>(&mut self, writer: &mut W) -> io::Result<bool> {
        let range = self.range_from(self.consumed);
        self.consumed = self.data.len();
        self.data.write_into(writer, range)
    }
}

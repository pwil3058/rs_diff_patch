// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::range::{Len, Range};
use std::collections::HashMap;
use std::io;
use std::io::Write;

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

#[derive(Debug, Default)]
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
    /// use diff_lib_ng::data::*;
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
    /// use diff_lib_ng::data::*;
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
}

impl<T: PartialEq> Data<T> {
    /// Convenience function
    pub fn range_from(&self, from: usize) -> Range {
        Range(from, self.0.len())
    }

    pub fn subsequence(&self, range: Range) -> impl DoubleEndedIterator<Item = &T> {
        self.0[range.0..range.1].iter()
    }

    pub fn has_subsequence_at(&self, subsequence: &[T], at: usize) -> bool {
        if at < self.0.len() && self.0.len() - at >= subsequence.len() {
            subsequence
                .iter()
                .zip(self.0[at..].iter())
                .all(|(b, a)| a == b)
        } else {
            false
        }
    }
}

pub trait DataIfce<T: PartialEq>: Len + GenerateContentIndices<T> + WriteDataInto {
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

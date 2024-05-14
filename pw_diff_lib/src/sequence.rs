// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::range::{Len, Range};
use std::collections::HashMap;

pub trait Sequence<T: PartialEq>: Len {
    fn items<'a>(&'a self) -> impl DoubleEndedIterator<Item = &'a T>
    where
        T: 'a;
    fn subsequence<'a>(&'a self, range: Range) -> impl DoubleEndedIterator<Item = &'a T>
    where
        T: 'a;

    fn range_from(&self, from: usize) -> Range {
        Range(from, self.len())
    }
}

#[derive(Debug, Clone)]
pub struct StringSequence(Box<[String]>);

impl Len for StringSequence {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<String> for StringSequence {
    fn from(text: String) -> Self {
        Self(text.split_inclusive('\n').map(|s| s.to_string()).collect())
    }
}

impl From<&str> for StringSequence {
    fn from(arg: &str) -> Self {
        Self::from(arg.to_string())
    }
}

impl Sequence<String> for StringSequence {
    fn items<'a>(&'a self) -> impl DoubleEndedIterator<Item = &'a String>
    where
        String: 'a,
    {
        self.0.iter()
    }

    fn subsequence<'a>(&'a self, range: Range) -> impl DoubleEndedIterator<Item = &'a String>
    where
        String: 'a,
    {
        debug_assert!(range.1 <= self.0.len());
        self.0[range.0..range.1].iter()
    }
}

#[derive(Debug, Clone)]
pub struct ByteSequence(Box<[u8]>);

impl Len for ByteSequence {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<Vec<u8>> for ByteSequence {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes.into_boxed_slice())
    }
}

impl Sequence<u8> for ByteSequence {
    fn items<'a>(&'a self) -> impl DoubleEndedIterator<Item = &'a u8>
    where
        String: 'a,
    {
        self.0.iter()
    }

    fn subsequence<'a>(&'a self, range: Range) -> impl DoubleEndedIterator<Item = &'a u8>
    where
        String: 'a,
    {
        debug_assert!(range.1 <= self.0.len());
        self.0[range.0..range.1].iter()
    }
}

pub trait ContentItemIndices<T: PartialEq> {
    fn new(sequence: impl Sequence<T>) -> Self;
    fn indices(&self, item: &T) -> Option<&Vec<usize>>;
}

#[derive(Debug, Default)]
pub struct StringItemIndices(HashMap<String, Vec<usize>>);

impl ContentItemIndices<String> for StringItemIndices {
    fn new(sequence: impl Sequence<String>) -> Self {
        let mut map = HashMap::<String, Vec<usize>>::new();
        for (index, line) in sequence.items().enumerate() {
            if let Some(vec) = map.get_mut(line) {
                vec.push(index)
            } else {
                map.insert(line.to_string(), vec![index]);
            }
        }

        Self(map)
    }

    fn indices(&self, item: &String) -> Option<&Vec<usize>> {
        self.0.get(item)
    }
}

#[derive(Debug)]
pub struct ByteItemIndices(pub [Vec<usize>; 256]);

impl ContentItemIndices<u8> for ByteItemIndices {
    /// Generate the content to index mechanism for the given `Sequence`
    ///
    /// Example:
    /// ```
    /// use pw_diff_lib::sequence::*;
    /// let sequence = ByteSequence::from(vec![0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
    /// let indices = ByteItemIndices::new(sequence);
    /// assert_eq!(indices.indices(&0u8),Some( &vec![0usize,17]));
    /// assert_eq!(indices.indices(&16u8),Some( &vec![16usize,33]));
    /// assert_eq!(indices.indices(&17u8),None);
    /// ```
    fn new(sequence: impl Sequence<u8>) -> Self {
        const ARRAY_REPEAT_VALUE: Vec<usize> = Vec::<usize>::new();
        let mut indices = [ARRAY_REPEAT_VALUE; 256];
        for (index, byte) in sequence.items().enumerate() {
            indices[*byte as usize].push(index);
        }
        Self(indices)
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

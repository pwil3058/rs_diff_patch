// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use std::collections::HashMap;
use std::ops::Deref;

pub struct Seq<T: PartialEq>(Box<[T]>);

impl<T: PartialEq> Deref for Seq<T> {
    type Target = Box<[T]>;

    fn deref(&self) -> &Self::Target {
        &self.0
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

pub trait ContentItemIndices<T: PartialEq> {
    fn from(sequence: &Seq<T>) -> Self;
    fn indices(&self, item: &T) -> Option<&Vec<usize>>;
}

#[derive(Debug, Default)]
pub struct StringItemIndices(HashMap<String, Vec<usize>>);

impl ContentItemIndices<String> for StringItemIndices {
    fn from(sequence: &Seq<String>) -> Self {
        let mut map = HashMap::<String, Vec<usize>>::new();
        for (index, line) in sequence.iter().enumerate() {
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
    /// let sequence = Seq::Byte::from(vec![0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
    /// let indices = ByteItemIndices::new(sequence);
    /// assert_eq!(indices.indices(&0u8),Some( &vec![0usize,17]));
    /// assert_eq!(indices.indices(&16u8),Some( &vec![16usize,33]));
    /// assert_eq!(indices.indices(&17u8),None);
    /// ```
    fn from(sequence: &Seq<u8>) -> Self {
        const ARRAY_REPEAT_VALUE: Vec<usize> = Vec::<usize>::new();
        let mut indices = [ARRAY_REPEAT_VALUE; 256];
        for (index, byte) in sequence.iter().enumerate() {
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

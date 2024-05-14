// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

use crate::range::Range;
use std::collections::HashMap;

pub trait Sequence<T: PartialEq> {
    fn items<'a>(&'a self) -> impl DoubleEndedIterator<Item = &'a T>
    where
        T: 'a;
    fn subsequence<'a>(&'a self, range: Range) -> impl DoubleEndedIterator<Item = &'a T>
    where
        T: 'a;
}

pub struct StringSequence(Box<[String]>);

pub struct ByteSequence(Box<[u8]>);

pub trait ContentItemIndices<T: PartialEq> {
    fn new(sequence: impl Sequence<T>) -> Self;
    fn indices(&self, item: &T) -> Option<&Vec<usize>>;
}

#[derive(Debug, Default)]
pub struct StringIndices(HashMap<String, Vec<usize>>);

impl ContentItemIndices<String> for StringIndices {
    fn new(sequence: StringSequence) -> Self {
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
pub struct ByteIndices(pub [Vec<usize>; 256]);

impl ContentItemIndices<u8> for crate::ByteIndices {
    // Generate the content to index mechanism for the given `Sequence`
    //
    // Example:
    // ```
    // use pw_diff_lib::sequence::*;
    // let data = ByteSequence::from(vec![0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]);
    // let indices = data.generate_content_indices();
    // assert_eq!(indices.indices(&0u8),Some( &vec![0usize,17]));
    // assert_eq!(indices.indices(&16u8),Some( &vec![16usize,33]));
    // assert_eq!(indices.indices(&17u8),None);
    // ```
    fn new(sequence: ByteSequence) -> Self {
        const ARRAY_REPEAT_VALUE: Vec<usize> = Vec::<usize>::new();
        let mut indices = [ARRAY_REPEAT_VALUE; 256];
        for (index, byte) in sequence.items().enumerate() {
            indices[*byte as usize].push(index);
        }
        crate::ByteIndices(indices)
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

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

pub trait ContentItemIndices<T: PartialEq> {
    fn new(sequence: impl Sequence<T>) -> Self;
    fn indices(&self, key: &T) -> Option<&Vec<usize>>;
}

#[derive(Debug, Default)]
pub struct StringIndices(HashMap<String, Vec<usize>>);

impl ContentItemIndices<String> for StringIndices {
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

    fn indices(&self, key: &String) -> Option<&Vec<usize>> {
        self.0.get(key)
    }
}

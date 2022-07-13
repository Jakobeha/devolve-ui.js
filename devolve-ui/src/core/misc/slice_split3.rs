//! Split at an index and return mutable references to the elements before, after, and the element itself.

use std::cmp::Ordering;
use std::iter::{Iterator, ExactSizeIterator};

pub struct IterMutSplit3s<'a, T> {
    slice: &'a mut [T],
    index: usize
}

pub trait SliceSplit3<T> {
    fn split3_mut(&mut self, index: usize) -> (&mut [T], &mut T, &mut [T]);
    fn iter_mut_split3s(&mut self) -> IterMutSplit3s<'_, T>;
}

impl<T> SliceSplit3<T> for [T] {
    fn split3_mut(&mut self, index: usize) -> (&mut [T], &mut T, &mut [T]) {
        let (before, current_and_after) = self.split_at_mut(index);
        let (current, after) = current_and_after.split_first_mut().unwrap();
        (before, current, after)
    }

    fn iter_mut_split3s(&mut self) -> IterMutSplit3s<'_, T> {
        IterMutSplit3s::new(self)
    }
}

impl<'a, T> IterMutSplit3s<'a, T> {
    fn new(slice: &'a mut [T]) -> IterMutSplit3s<'a, T> {
        IterMutSplit3s {
            slice,
            index: 0
        }
    }
}

impl<'a, T> Iterator for IterMutSplit3s<'a, T> {
    type Item = (&'a mut [T], &'a mut T, &'a mut [T]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.slice.len() {
            None
        } else {
            let split3 = self.slice.split3_mut(self.index);
            self.index += 1;
            Some(split3)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.slice.len()))
    }

    fn count(&mut self) -> usize {
        self.slice.len()
    }
}

impl<'a, T> ExactSizeIterator for IterMutSplit3s<'a, T> {}
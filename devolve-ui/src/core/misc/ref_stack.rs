//! A vector where each element further down has a shorter lifetime than the top, which has lifetime 'a.
//! Internally this stores the elements as pointers, but has invariants to guarantee (unproven) safety.

use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Debug, PartialEq)]
pub struct RefStack<'a, T>(
    Vec<*mut T>,
    PhantomData<&'a ()>,
);

impl <T> RefStack<'static, T> {
    /// Create a stack with no elements and a static lifetime, which means that
    /// every element pushed will be popped before the stack is dropped.
    pub fn new() -> Self {
        RefStack(Vec::new(), PhantomData)
    }
}

impl <'a, T> RefStack<'a, T> {
    /// Push an element, then run fun, then pop.
    /// The fact that this is a function guarantees that `elem` will not be dropped while `fun` is run,
    /// ensuring that storing them as pointers is safe.
    pub fn with_push<R>(&mut self, elem: &mut T, fun: impl FnOnce(&mut RefStack<'_, T>) -> R) -> R {
        self.0.push(elem as *mut T);
        let result = fun(self);
        self.0.pop();
        result
    }

    /// Get the top item
    pub fn top_mut<'b>(&'b mut self) -> Option<&'b mut &'a mut T> {
        self.top_mut_assoc().map(|(map, ())| map)
    }

    /// Iterate from top to bottom
    pub fn iter_rev(&self) -> impl Iterator<Item=&T> {
        self.0.iter().rev().map(|elem| unsafe { &**elem })
    }

    /// Iterate from top to bottom
    pub fn iter_mut_rev(&mut self) -> impl Iterator<Item=&mut T> {
        self.0.iter_mut().rev().map(|elem| unsafe { &mut **elem })
    }
}

impl <'a, K, V> FromIterator<&'a T> for RefStack<'a, T> {
    /// Create a stack from a vector of references. Since the references must outlive the stack itself,
    /// this ensures that storing them as pointers is safe.
    fn from_iter<I: IntoIterator<Item=&'a mut HashMap<K, V>>>(iter: I) -> Self {
        RefStack(iter.into_iter().map(|elem| elem as *mut _).collect(), PhantomData)
    }
}
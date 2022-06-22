//! Combine multiple references to hash maps to form a larger map.
//! If they share keys, the top (last pushed) map gets priority.
//! You can optionally associate a value to each hash map,
//! which will be borrowed from getting any element in that map.

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use replace_with::replace_with_or_abort_and_return;
use crate::core::misc::ref_stack::MutStack;

#[derive(Debug, PartialEq)]
pub struct HashMapMutStack<'a, K, V>(MutStack<'a, HashMap<K, V>>);

pub struct HashMapWithAssocMutStack<'a, K, V, Assoc>(
    Vec<(*mut HashMap<K, V>, *mut Assoc)>,
    PhantomData<&'a ()>,
);

impl <K, V> HashMapMutStack<'static, K, V> {
    pub fn new() -> Self {
        HashMapMutStack(MutStack::new())
    }
}

impl <K, V, Assoc> HashMapWithAssocMutStack<'static, K, V, Assoc> {
    pub fn new() -> Self {
        HashMapWithAssocMutStack(Vec::new(), PhantomData)
    }
}

static UNUSED_ASSOC: () = ();

impl <'a, K, V> HashMapMutStack<'a, K, V> {
    pub fn with_push<R>(&mut self, map: &mut HashMap<K, V>, fun: impl FnOnce(&mut HashMapMutStack<'_, K, V>) -> R) -> R {
        self.0.with_push(map, |stack| {
            replace_with_or_abort_and_return(stack, |stack| {
                let mut map_stack = HashMapMutStack(stack);
                let result = fun(&mut map_stack);
                (result, map_stack.0)
            })
        })
    }

    pub fn top_mut<'b>(&'b mut self) -> Option<&'b mut &'a mut HashMap<K, V>> {
        self.0.top_mut()
    }
}

impl <'a, K, V, Assoc> HashMapWithAssocMutStack<'a, K, V, Assoc> {
    pub fn with_push<R>(&mut self, map: &mut HashMap<K, V>, assoc: &mut Assoc, fun: impl FnOnce(&mut HashMapWithAssocMutStack<'_, K, V, Assoc>) -> R) -> R {
        self.0.push((map as *mut HashMap<K, V>, assoc as *mut Assoc));
        let result = fun(self);
        self.0.pop();
        result
    }

    pub fn top_mut<'b>(&'b mut self) -> Option<(&'b mut &'a mut HashMap<K, V>, &'b mut &'a mut Assoc)> {
        unsafe { mem::transmute(self.0.last_mut()) }
    }


    /// Iterate from bottom to top
    fn iter(&self) -> impl Iterator<Item=(&HashMap<K, V>, &Assoc)> {
        self.0.iter().map(|(elem, assoc)| unsafe { (&**elem, &**assoc) })
    }
}

impl <'a, K: Eq + Hash, V> HashMapMutStack<'a, K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        for map in self.0.iter_rev() {
            if let Some(value) = map.get(key) {
                return Some(value);
            }
        }
        None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        for map in self.0.iter_mut_rev() {
            if let Some(value) = map.get_mut(key) {
                return Some(value);
            }
        }
        None
    }
}

impl <'a, K: Eq + Hash, V, Assoc> HashMapWithAssocMutStack<'a, K, V, Assoc> {
    pub fn get(&self, key: &K) -> Option<(&V, &Assoc)> {
        for (map, assoc) in self.0.iter().rev() {
            let map = unsafe { &**map };
            let assoc = unsafe { &**assoc };
            if let Some(value) = map.get(key) {
                return Some((value, assoc));
            }
        }
        None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<(&mut V, &mut Assoc)> {
        for (map, assoc) in self.0.iter_mut().rev() {
            let map = unsafe { &mut **map };
            let assoc = unsafe { &mut **assoc };
            if let Some(value) = map.get_mut(key) {
                return Some((value, assoc));
            }
        }
        None
    }
}

impl <'a, K: 'a, V: 'a> FromIterator<&'a mut HashMap<K, V>> for HashMapMutStack<'a, K, V> {
    fn from_iter<I: IntoIterator<Item=&'a mut HashMap<K, V>>>(iter: I) -> Self {
        HashMapMutStack(MutStack::from_iter(iter))
    }
}

impl <'a, K: 'a, V: 'a, Assoc> FromIterator<(&'a mut HashMap<K, V>, &'a mut Assoc)> for HashMapWithAssocMutStack<'a, K, V, Assoc> {
    fn from_iter<I: IntoIterator<Item=(&'a mut HashMap<K, V>, &'a mut Assoc)>>(iter: I) -> Self {
        HashMapWithAssocMutStack(iter.into_iter().map(|(elem, assoc)| (elem as *mut _, assoc as *mut _)).collect(), PhantomData)
    }
}

impl <'a, K: Debug, V: Debug, Assoc: Debug> Debug for HashMapWithAssocMutStack<'a, K, V, Assoc> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl <'a, 'b, K: Eq + Hash, V: PartialEq<V>, AssocA: PartialEq<AssocB>, AssocB> PartialEq<HashMapWithAssocMutStack<'b, K, V, AssocB>> for HashMapWithAssocMutStack<'a, K, V, AssocA> {
    fn eq(&self, other: &HashMapWithAssocMutStack<'b, K, V, AssocB>) -> bool {
        self.iter().eq(other.iter())
    }
}
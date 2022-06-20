//! Combine multiple references to hash maps to form a larger map.
//! If they share keys, the top (last pushed) map gets priority.

use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;

#[derive(Debug, PartialEq)]
pub struct HashMapMutStack<'a, K, V>(
    /// Vector where each element further down has a shorter lifetime than the top, which has lifetime 'a
    Vec<*mut HashMap<K, V>>,
    PhantomData<&'a ()>,
);

impl <K, V> HashMapMutStack<'static, K, V> {
    pub fn new() -> Self {
        HashMapMutStack(Vec::new(), PhantomData)
    }
}

impl <'a, K, V> HashMapMutStack<'a, K, V> {
    pub fn with_push<R>(&mut self, map: &mut HashMap<K, V>, fun: impl FnOnce(&mut HashMapMutStack<K, V>) -> R) -> R {
        self.0.push(map as *mut HashMap<K, V>);
        let result = fun(self);
        self.0.pop();
        result
    }

    pub fn top_mut<'b>(&'b mut self) -> Option<&'b mut &'a mut HashMap<K, V>> {
        unsafe { mem::transmute(self.0.last_mut()) }
    }
}

impl <'a, K: Eq + Hash, V> HashMapMutStack<'a, K, V> {
    pub fn get(&self, key: &K) -> Option<&V> {
        for map in self.0.iter().rev() {
            let map = unsafe { &**map };
            if let Some(value) = map.get(key) {
                return Some(value);
            }
        }
        None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        for map in self.0.iter_mut().rev() {
            let map = unsafe { &mut **map };
            if let Some(value) = map.get_mut(key) {
                return Some(value);
            }
        }
        None
    }
}

impl <'a, K, V> FromIterator<&'a mut HashMap<K, V>> for HashMapMutStack<'a, K, V> {
    fn from_iter<I: IntoIterator<Item=&'a mut HashMap<K, V>>>(iter: I) -> Self {
        HashMapMutStack(iter.into_iter().map(|elem| elem as *mut _).collect(), PhantomData)
    }
}
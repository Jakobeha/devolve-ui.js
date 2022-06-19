//! Combine multiple references to hash maps to form a larger map.
//! If they share keys, the top (last pushed) map gets priority.

use std::collections::HashMap;

pub struct HashMapMutStack<'a, K, V>(Vec<&'a mut HashMap<K, V>>);

impl <'a, K, V> HashMapMutStack<'a, K, V> {
    pub fn new() -> Self {
        HashMapMutStack(Vec::new())
    }

    pub fn push(&mut self, map: &'a mut HashMap<K, V>) {
        self.0.push(map);
    }

    pub fn pop(&mut self) -> Option<&'a mut HashMap<K, V>> {
        self.0.pop()
    }

    pub fn top_mut<'b>(&'b mut self) -> Option<&'b mut &'a mut HashMap<K, V>> where 'a: 'b {
        self.0.last_mut()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        for map in self.0.iter().rev() {
            if let Some(value) = map.get(key) {
                return Some(value);
            }
        }
        None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        for map in self.0.iter_mut().rev() {
            if let Some(value) = map.get_mut(key) {
                return Some(value);
            }
        }
        None
    }
}
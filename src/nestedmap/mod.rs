//! NestedMap is an experimental lockfree map that is not ready for produuction use yet.
//! No guarantees are made at the moment. Use at your own risk.

mod raw;

#[cfg(test)]
mod tests;

use raw::{Table, Bucket, Entry};
pub use raw::TableRef;
use crossbeam_epoch::Owned;
use std::hash::Hash;

pub struct NestedMap<K: Hash + Eq, V> {
    root: Table<K, V>,
}

impl<'a, K: 'a + Hash + Eq, V: 'a> NestedMap<K, V> {
    pub fn new() -> Self {
        Self {
            root: Table::empty(),
        }
    }

    #[inline]
    pub fn insert(&self, key: K, value: V) {
        let bucket = Owned::new(Bucket::Leaf(Entry { key, value }));
        self.root.insert(bucket);
    }

    #[inline]
    pub fn get(&'a self, key: &K) -> Option<TableRef<'a, K, V>> {
        self.root.get(key)
    }

    #[inline]
    pub fn remove(&self, key: &K) {
        self.root.remove(key);
    }
}

impl<'a, K: 'a + Hash + Eq, V: 'a> Default for NestedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

//! NestedMap is an experimental lockfree map that is not ready for use yet.
//! No guarantees are made at the moment. Use at your own risk.

mod raw;

#[cfg(test)]
mod tests;

use crate::uniform_allocator::UniformAllocator;
use crate::util::UniformAllocExt;
use ccl_crossbeam_epoch::{self as epoch, Guard, Owned};
use rand::prelude::*;
pub use raw::{TableRef, TableIter};
use raw::{Bucket, Entry, Table};
use std::hash::Hash;
use std::sync::Arc;
use std::rc::Rc;

#[inline]
pub fn aquire_guard() -> Guard {
    epoch::pin()
}

pub struct NestedMap<K: Hash + Eq, V> {
    root: Table<K, V>,
}

impl<'a, K: 'a + Hash + Eq, V: 'a> NestedMap<K, V> {
    pub fn new() -> Self {
        Self {
            root: Table::empty(Arc::new(UniformAllocator::default())),
        }
    }

    #[inline]
    pub fn insert(&self, key: K, value: V) {
        let guard = &epoch::pin();
        self.insert_with_guard(key, value, guard);
    }

    #[inline]
    pub fn insert_with_guard(&self, key: K, value: V, guard: &Guard) {
        let tag: u8 = rand::thread_rng().gen();

        let bucket = Owned::uniform_alloc(
            self.root.allocator(),
            tag as usize,
            Bucket::Leaf(tag, Entry { key, value }),
        );
        self.root.insert(bucket, guard);
    }

    #[inline]
    pub fn get(&'a self, key: &K) -> Option<TableRef<'a, K, V>> {
        let guard = epoch::pin();
        self.get_with_guard(key, guard)
    }

    #[inline]
    pub fn get_with_guard(&'a self, key: &K, guard: Guard) -> Option<TableRef<'a, K, V>> {
        self.root.get(key, guard)
    }

    #[inline]
    pub fn remove(&self, key: &K) {
        let guard = &epoch::pin();
        self.remove_with_guard(key, guard);
    }

    #[inline]
    pub fn remove_with_guard(&self, key: &K, guard: &Guard) {
        self.root.remove(key, guard);
    }

    #[inline]
    pub fn contains_key(&self, key: &K) -> bool {
        let guard = epoch::pin();
        self.root.contains_key(key, guard)
    }

    #[inline]
    pub fn iter(&'a self) -> TableIter<'a, K, V> {
        let guard = Rc::new(epoch::pin());
        self.root.iter(guard)
    }
}

impl<'a, K: 'a + Hash + Eq, V: 'a> Default for NestedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

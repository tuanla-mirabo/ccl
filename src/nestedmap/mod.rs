//! NestedMap is an experimental lockfree map that is not ready for use yet.
//! No guarantees are made at the moment. Use at your own risk.

mod raw;

#[cfg(test)]
mod tests;

use crate::uniform_allocator::UniformAllocator;
use crate::util::UniformAllocExt;
use crossbeam_epoch::{self as epoch, Guard, Owned};
use rand::prelude::*;
pub use raw::TableRef;
use raw::{Bucket, Entry, Table};
use std::hash::Hash;
use std::sync::Arc;

#[inline]
pub fn aquire_guard() -> Guard {
    epoch::pin()
}

pub struct NestedMap<K: Hash + Eq, V> {
    root: Table<K, V>,
}

impl<'a, K: 'a + Hash + Eq, V: 'a> NestedMap<K, V> {
    pub fn new() -> Self {
        let allocator_pool_count = num_cpus::get() * 4;

        Self {
            root: Table::empty(Arc::new(UniformAllocator::new(allocator_pool_count))),
        }
    }

    #[inline]
    pub fn insert(&self, key: K, value: V) {
        let guard = &epoch::pin();
        self.insert_with_guard(key, value, guard);
    }

    #[inline]
    pub fn insert_with_guard(&self, key: K, value: V, guard: &Guard) {
        let alloc_tag = rand::thread_rng().gen::<u8>();
        let bucket = Owned::uniform_alloc(
            self.root.allocator(),
            alloc_tag as usize,
            Bucket::Leaf(Entry {
                key,
                alloc_tag,
                value,
            }),
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
}

impl<'a, K: 'a + Hash + Eq, V: 'a> Default for NestedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

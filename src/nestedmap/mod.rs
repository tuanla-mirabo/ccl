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
use raw::{Bucket, Entry as RawEntry, Table};
use std::hash::Hash;
use std::sync::Arc;
use std::rc::Rc;
use std::fmt;

// TO-DO: fix vanishing items when inserting concurrent from multiple threads

pub struct OccupiedEntry<'a, K: Hash + Eq, V> {
    map: &'a NestedMap<K, V>,
    guard: Guard,
    r: TableRef<'a, K, V>,
    key: K,
}

impl<'a, K: Hash + Eq, V> OccupiedEntry<'a, K, V> {
    #[inline]
    pub fn new(guard: Guard, map: &'a NestedMap<K, V>, r: TableRef<'a, K, V>, key: K) -> Self {
        Self {
            map,
            guard,
            r,
            key,
        }
    }

    #[inline]
    pub fn key(&self) -> &K {
        self.r.key()
    }

    #[inline]
    pub fn remove(self) {
        self.map.remove_with_guard(self.r.key(), &self.guard);
    }

    #[inline]
    pub fn get(&self) -> &V {
        self.r.value()
    }

    #[inline]
    pub fn insert(self, value: V) {
        self.map.insert_with_guard(self.key, value, &self.guard);
    }

    #[inline]
    pub fn into_ref(self) -> TableRef<'a, K, V> {
        self.r
    }
}

pub struct VacantEntry<'a, K: Hash + Eq, V> {
    map: &'a NestedMap<K, V>,
    guard: Guard,
    key: K,
}

impl<'a, K: Hash + Eq, V> VacantEntry<'a, K, V> {
    #[inline]
    pub fn new(guard: Guard, map: &'a NestedMap<K, V>, key: K) -> Self {
        Self {
            map,
            guard,
            key,
        }
    }

    #[inline]
    pub fn insert(self, value: V) {
        self.map.insert_with_guard(self.key, value, &self.guard);
    }

    #[inline]
    pub fn into_key(self) -> K {
        self.key
    }

    #[inline]
    pub fn key(&self) -> &K {
        &self.key
    }
}

impl<'a, K: Hash + Eq + Clone, V> VacantEntry<'a, K, V> {
    #[inline]
    pub fn insert_with_ret(self, value: V) -> (&'a NestedMap<K, V>, Guard, K) {
        self.map.insert_with_guard(self.key.clone(), value, &self.guard);
        (self.map, self.guard, self.key)
    }
}

pub enum Entry<'a, K: Hash + Eq, V> {
    Occupied(OccupiedEntry<'a, K, V>),
    Vacant(VacantEntry<'a, K, V>),
}

impl<'a, K: Hash + Eq, V> Entry<'a, K, V> {
    #[inline]
    pub fn is_occupied(&self) -> bool {
        if let Entry::Occupied(_) = self {
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn into_occupied(self) -> Option<OccupiedEntry<'a, K, V>> {
        if let Entry::Occupied(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[inline]
    pub fn is_vacant(&self) -> bool {
        if let Entry::Vacant(_) = self {
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn into_vacant(self) -> Option<VacantEntry<'a, K, V>> {
        if let Entry::Vacant(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[inline]
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(v) => v.key(),
            Entry::Vacant(v) => v.key(),
        }
    }

    #[inline]
    pub fn and_inspect<F: FnOnce(&V)>(self, f: F) -> Self {
        if let Entry::Occupied(occupied) = &self {
            f(occupied.get());
        }

        self
    }
}

impl<'a, K: Hash + Eq + Clone, V> Entry<'a, K, V>  {
    pub fn or_insert(self, default: V) -> TableRef<'a, K, V> {
        match self {
            Entry::Occupied(occupied) => occupied.into_ref(),
            Entry::Vacant(vacant) => {
                let (map, guard, key) = vacant.insert_with_ret(default);
                map.get_with_guard(&key, guard).expect("this should never happen; nestedmap entry or_insert")
            }
        }
    }

    pub fn or_insert_with<F: FnOnce() -> V>(self, default: F) -> TableRef<'a, K, V> {
        match self {
            Entry::Occupied(occupied) => occupied.into_ref(),
            Entry::Vacant(vacant) => {
                let (map, guard, key) = vacant.insert_with_ret(default());
                map.get_with_guard(&key, guard).expect("this should never happen; nestedmap entry or_insert")
            }
        }
    }
}

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
            Bucket::Leaf(tag, RawEntry { key, value }),
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

    #[inline]
    pub fn len(&self) -> usize {
        let guard = &epoch::pin();
        self.root.len(guard)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn entry(&'a self, key: K) -> Entry<'a, K, V> {
        let guard = epoch::pin();

        match self.get(&key) {
            None => Entry::Vacant(VacantEntry::new(guard, self, key)),
            Some(r) => Entry::Occupied(OccupiedEntry::new(guard, self, r, key)),
        }
    }
}

impl<'a, K: 'a + Hash + Eq, V: 'a> Default for NestedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, K: 'a + Hash + Eq, V: 'a> fmt::Debug for NestedMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "NestedMap {{}}")
    }
}

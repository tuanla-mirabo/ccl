use std::sync::atomic::Ordering;
use std::hash::Hash;
use crate::util;
use crossbeam_epoch::{self as epoch, Atomic, Owned, Shared, Guard};
use std::ops::Deref;
use std::mem;

#[allow(dead_code)]
struct Item<K: Hash + Eq, V> {
    key: K,
    value: V,
}

enum Entry<K: Hash + Eq, V> {
    Empty,
    Occupied(Item<K, V>),
}

/// A very crude concurrent lockfree hashmap with no collision resolution or resizing.
pub struct CrudeHashMap<K: Hash + Eq, V> {
    table: Box<[Atomic<Entry<K, V>>]>,
}

impl<'a, K: 'a + Hash + Eq, V: 'a> CrudeHashMap<K, V> {
    pub fn new(capacity: usize) -> Self {
        let capacity = util::round_pow2(capacity);

        Self {
            table: (0..capacity).map(|_| Atomic::new(Entry::Empty)).collect::<Vec<_>>().into_boxed_slice(),
        }
    }

    pub fn insert(&self, key: K, value: V) {
        let hash = util::hash(&key);
        let idx = hash as usize % self.table.len();
        let slot = &self.table[idx];
        let guard = &epoch::pin();
        let new = Owned::new(Entry::Occupied(Item { key, value }));
        let _old = slot.swap(new, Ordering::SeqCst, guard);
    }

    pub fn get(&'a self, key: &K) -> Option<MapRef<V>> {
        let hash = util::hash(&key);
        let idx = hash as usize % self.table.len();
        let slot = &self.table[idx];
        let guard = epoch::pin();

        let fake_guard = unsafe { &*(0 as *const u8 as *const Guard) };

        let sharedptr: Shared<'a, Entry<K, V>> = slot.load(Ordering::SeqCst, fake_guard);

        let entry: &'a Entry<K, V> = unsafe { sharedptr.as_ref()? };

        match entry {
            Entry::Empty => None,
            Entry::Occupied(ref item) => {
                Some(MapRef {
                    guard: Some(guard),
                    ptr: &item.value,
                })
            }
        }
    }

    pub fn remove(&'a self, key: &K) {
        let hash = util::hash(&key);
        let idx = hash as usize % self.table.len();
        let slot = &self.table[idx];
        let guard = &epoch::pin();
        let sharedptr = slot.load(Ordering::SeqCst, guard);
        unsafe { guard.defer_destroy(sharedptr); }
    }
}

pub struct MapRef<'a, V> {
    guard: Option<epoch::Guard>,
    ptr: &'a V,
}

impl<'a, V> Drop for MapRef<'a, V> {
    fn drop(&mut self) {
        let guard = self.guard.take();
        mem::drop(guard);
    }
}

impl<'a, V> Deref for MapRef<'a, V> {
    type Target = V;

    fn deref(&self) -> &V {
        self.ptr
    }
}

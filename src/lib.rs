#![allow(dead_code, unused_imports)]

use swym::thread_key;
use swym::tcell::TCell;
use std::hash::Hash;
use std::marker::PhantomData;

pub const HASHMAP_INIT_CAPACITY_LOOKUP_BITS: usize = 8;

fn init_storage<V: Send>(capacity: usize) -> Vec<TCell<Entry<V>>> {
    (0..capacity).map(|_| TCell::new(Entry::Vacant)).collect()
}

fn compute_index(hash: u32, lookup_bits_count: usize) -> usize {
    let shift = 32 - lookup_bits_count;
    (hash >> shift) as usize
}

pub enum Entry<V: Send> {
    Vacant,
    Occupied(V),
}

pub struct TCHashMap<K, V>
where
    K: Hash + ?Sized,
    V: Send,
{
    storage: Vec<TCell<Entry<V>>>,
    lookup_bits_count: usize,
    capacity: usize,
    phantom: PhantomData<K>,
}

impl<K, V> TCHashMap<K, V>
where
    K: Hash + ?Sized,
    V: Send + 'static + Clone,
{
    pub fn new() -> Self {
        let capacity = 2_usize.pow(HASHMAP_INIT_CAPACITY_LOOKUP_BITS as u32);

        Self {
            storage: init_storage::<V>(capacity),
            lookup_bits_count: HASHMAP_INIT_CAPACITY_LOOKUP_BITS,
            capacity,
            phantom: PhantomData,
        }
    }

    pub fn insert(&self, k: &K, v: V) {
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, self.lookup_bits_count);

        thread_key.rw(|tx| {
            self.storage[index].set(tx, Entry::Occupied(v.clone()))?;
            Ok(())
        });
    }
}

impl<K, V> Default for TCHashMap<K, V>
where
    K: Hash + ?Sized,
    V: Send + 'static + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

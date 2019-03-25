#![allow(dead_code, unused_imports)]

use swym::thread_key;
use swym::tcell::{TCell, Ref};
use std::hash::Hash;
use std::marker::PhantomData;
use swym::{ReadTx, RWTx};
use swym::tx::Ordering;
//use std::borrow::Borrow;

pub const HASHMAP_INIT_CAPACITY_LOOKUP_BITS: usize = 8;

fn init_storage<V: Send + Clone>(capacity: usize) ->  Vec<TCell<Box<Entry<V>>>> {
    (0..capacity).map(|_| TCell::new(Box::new(Entry::Vacant))).collect()
}

fn compute_index(hash: u32, lookup_bits_count: usize) -> usize {
    let shift = 32 - lookup_bits_count;
    (hash >> shift) as usize
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Entry<V: Send + Sized + Clone> {
    Vacant,
    Occupied(V),
}

pub struct TCHashMap<K, V>
where
    K: Hash + ?Sized,
    V: Send + Clone,
{
    storage: Vec<TCell<Box<Entry<V>>>>,
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
            self.storage[index].set(tx, Box::new(Entry::Occupied(v.clone())))?;
            Ok(())
        });
    }

    pub fn get(&self, k: &K) -> Entry<V> { // View<V, ReadTx> &TCell<Entry<V>>
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, self.lookup_bits_count);

        //self.storage[index].borrow()
        //thread_key.read(|tx| self.storage[index].view(tx))
        //thread_key.read(|tx| Ok(self.storage[index].borrow(tx, Ordering::Read)?).
        let mut v = None;

        thread_key.read(|tx| {
            let inner: Result<Ref<Box<Entry<V>>>, ()> = Ok(self.storage[index].borrow(tx, Ordering::Read)?);
            if let Ok(inner) = inner {
                v = Some(*inner.clone());
            }

            Ok(())
        });

        v.expect("undefined wtf")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_assert() {
        let map = TCHashMap::new();
        map.insert("num", 17_i32);
        assert_eq!(map.get("num"), Entry::Occupied(17_i32));
    }
}

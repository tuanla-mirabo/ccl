//! tchashmap is a highly experimental hashmap based of transactional memory

use swym::thread_key;
use swym::tcell::{TCell, Ref};
use std::hash::Hash;
// use swym::{ReadTx, RWTx};
use swym::tx::{Borrow, Ordering};
use std::sync::atomic;
use parking_lot::RwLock;
use std::mem;

pub const HASHMAP_INIT_LOOKUP_BITS: usize = 4;
pub const HASHMAP_LOAD_THRESHOLD: f32 = 0.5;

#[inline]
fn init_storage<K: Send + Sized + Clone + Hash, V: Send + Sized + Clone>(capacity: usize) ->  Vec<TCell<Entry<K, V>>> {
    (0..capacity).map(|_| TCell::new(Entry::Vacant)).collect()
}

#[inline]
fn compute_index(hash: u32, lookup_bits_count: usize) -> usize {
    let shift = 32 - lookup_bits_count;
    (hash >> shift) as usize
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Entry<K: Send + Sized + Clone + Hash, V: Send + Sized + Clone> {
    Vacant,
    Occupied(K, V),
}

unsafe impl<K: Send + Sized + Clone + Hash, V: Send + Sized + Clone> Borrow for Entry<K, V> {}

struct TCHashMapRaw<K, V>
where
    K: Send + Clone + Hash + Sized,
    V: Send + Clone,
{
    pub storage: Vec<TCell<Entry<K, V>>>,
    pub len: atomic::AtomicUsize,
    pub lbc: usize,
    pub capacity: usize,
}

pub struct TCHashMap<K, V>
where
    K: Send + Clone + Hash + Sized,
    V: Send + Clone,
{
    inner: RwLock<TCHashMapRaw<K, V>>,
}

impl<K: 'static, V> TCHashMap<K, V>
where
    K: Send+ Clone + Hash + Sized,
    V: Send + 'static + Clone,
{
    pub fn new() -> Self {
        let capacity = 2_usize.pow(HASHMAP_INIT_LOOKUP_BITS as u32);

        Self {
            inner: RwLock::new(TCHashMapRaw {
                storage: init_storage::<K, V>(capacity),
                len: atomic::AtomicUsize::new(0),
                lbc: HASHMAP_INIT_LOOKUP_BITS,
                capacity,
            })
        }
    }

    pub fn resize(&self, lbc: usize) {
        let mut inner = self.inner.write();
        let new_capacity = 2_usize.pow(lbc as u32);

        inner.lbc = lbc;
        inner.capacity = new_capacity;

        let new_buf = init_storage(new_capacity);
        let old_buf = mem::replace(&mut inner.storage, new_buf);

        old_buf.into_iter().for_each(|cell| {
            if let Entry::Occupied(k, v) = cell.into_inner() {
                let hash = fxhash::hash32(&k);
                let index = compute_index(hash, lbc);
                println!("index: {} hash: {}", index, hash);
                inner.storage[index] = TCell::new(Entry::Occupied(k, v));
            }
        });
    }

    pub fn insert(&self, k: K, v: V) {
        let inner = self.inner.read();
        let len = inner.len.load(atomic::Ordering::Relaxed);
        let lbc = inner.lbc;
        let capacity = inner.capacity;
        drop(inner);

        println!("newlen: {}, thcap: {}", len + 1, (capacity as f32 * HASHMAP_LOAD_THRESHOLD) as usize);
        if len + 1 > (capacity as f32 * HASHMAP_LOAD_THRESHOLD) as usize {
            self.resize(lbc + 1);
        }

        let inner = self.inner.read();
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, inner.lbc);
        inner.len.fetch_add(1, atomic::Ordering::Relaxed);
        thread_key.rw(|tx| {
            inner.storage[index].set(tx, Entry::Occupied(k.clone(), v.clone()))?;
            Ok(())
        });
    }

    pub fn get_cloned(&self, k: &K) -> V {
        let inner = self.inner.read();
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, inner.lbc);
        let mut v = None;

        thread_key.read(|tx| {
            let inner: Result<Ref<Entry<K, V>>, ()> = Ok(inner.storage[index].borrow(tx, Ordering::Read)?);
            if let Ok(inner) = inner {
                v = Some(inner.clone());
            }

            Ok(())
        });

        if let Entry::Occupied(_, v) = v.expect("undefined error from swym") {
            v
        } else {
            panic!()
        }
    }
}

impl<K: 'static, V> Default for TCHashMap<K, V>
where
    K: Send + Clone + Hash + Sized,
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
        let n: u32 = 128;
        (0..n).for_each(|i| {
            map.insert(i, i + 9);
        });

        (0..n).for_each(|i| {
            assert_eq!(i + 9, map.get_cloned(&i));
        });
    }
}

//! tchashmap is a highly experimental hashmap based of transactional memory
//! TODO: collision handling (currently linear probling, robin hood), better api (closures, get, alter etc)

use swym::thread_key;
use swym::tcell::TCell;
use std::hash::Hash;
// use swym::{ReadTx, RWTx};
use swym::tx::{Borrow, Ordering};
use std::sync::atomic;
use parking_lot::RwLock;
use std::mem;

pub const HASHMAP_INIT_LOOKUP_BITS: usize = 4;
pub const HASHMAP_LOAD_THRESHOLD: f32 = 0.8;

#[inline]
fn init_storage<K: PartialEq + Send + Sized + Clone + Hash, V: Send + Sized + Clone>(capacity: usize) ->  Vec<TCell<Entry<K, V>>> {
    (0..capacity).map(|_| TCell::new(Entry::Vacant)).collect()
}

#[inline]
fn compute_index(hash: u32, lookup_bits_count: usize) -> usize {
    let shift = 32 - lookup_bits_count;
    (hash >> shift) as usize
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Entry<K: PartialEq + Send + Sized + Clone + Hash, V: Send + Sized + Clone> {
    Vacant,
    Tombstone,
    Occupied(K, V),
}

unsafe impl<K: PartialEq + Send + Sized + Clone + Hash, V: Send + Sized + Clone> Borrow for Entry<K, V> {}

struct TCHashMapRaw<K, V>
where
    K: PartialEq + Send + Clone + Hash + Sized,
    V: Send + Clone,
{
    pub storage: Vec<TCell<Entry<K, V>>>,
    pub len: atomic::AtomicUsize,
    pub lbc: usize,
    pub capacity: usize,
}

pub struct TCHashMap<K, V>
where
    K: PartialEq + Send + Clone + Hash + Sized,
    V: Send + Clone,
{
    inner: RwLock<TCHashMapRaw<K, V>>,
}

impl<K: 'static, V> TCHashMap<K, V>
where
    K: PartialEq + Send + Clone + Hash + Sized,
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

        if lbc != inner.lbc {
            let new_capacity = 2_usize.pow(lbc as u32);

            inner.lbc = lbc;
            inner.capacity = new_capacity;


            let new_buf = init_storage(new_capacity);
            let old_buf: Vec<_> = mem::replace(&mut inner.storage, new_buf).into_iter().map(|cell| cell.into_inner()).collect();
            let capacity = inner.capacity;
            let storage = &mut inner.storage;

            old_buf.into_iter().for_each(|entry| {
                if let Entry::Occupied(ref k , ref v) = entry {
                    let hash = fxhash::hash32(&k);
                    let mut index = compute_index(hash, lbc);
                    while match *storage[index].borrow_mut() {
                        Entry::Occupied(_, _) => {
                            index = index.wrapping_add(1) % capacity;
                            true
                        }

                        _ => {
                        storage[index] = TCell::new(Entry::Occupied(k.clone(), v.clone()));
                            false
                        }
                    } {}
                }
            });
        }
    }

    pub fn over_threshold(&self) -> (bool, usize) {
        let inner = self.inner.read();
        let len = inner.len.load(atomic::Ordering::Relaxed);
        let lbc = inner.lbc;
        let capacity = inner.capacity;

        if len + 1 > (capacity as f32 * HASHMAP_LOAD_THRESHOLD) as usize {
            (true, lbc + 1)
        } else {
            (false, lbc + 1)
        }
    }

    pub fn insert(&self, k: K, v: V) {
        let th_stat = self.over_threshold();
        if th_stat.0 {
            self.resize(th_stat.1);
        }

        let inner = self.inner.read();
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, inner.lbc);
        inner.len.fetch_add(1, atomic::Ordering::Relaxed);
        thread_key.rw(|tx| {
            let mut index = index;

            while match *inner.storage[index].borrow(tx, Ordering::Read)? {
                Entry::Vacant => {
                    inner.storage[index].set(tx, Entry::Occupied(k.clone(), v.clone()))?;
                    false
                }

                Entry::Occupied(ref key, _) => {
                    if key == &k {
                        inner.storage[index].set(tx, Entry::Occupied(k.clone(), v.clone()))?;
                        false
                    } else {
                        index = index.wrapping_add(1) % inner.capacity;
                        true
                    }
                }

                _ => {
                    index = index.wrapping_add(1) % inner.capacity;
                    true
                }
            } {}
            Ok(())
        });
    }

    pub fn get_cloned(&self, k: &K) -> Option<V> {
        let inner = self.inner.read();
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, inner.lbc);
        let mut v = None;

        thread_key.read(|tx| {
            let mut index = index;

            while match *inner.storage[index].borrow(tx, Ordering::Read)? {
                Entry::Vacant => {
                    false
                }

                Entry::Tombstone => {
                    unimplemented!();
                }

                Entry::Occupied(ref key, ref value) => {
                    if k == key {
                        v = Some(value.clone());
                        false
                    } else {
                        index = index.wrapping_add(1);
                        true
                    }
                }
            } {}

            Ok(())
        });

        v
    }
}

impl<K: 'static, V> Default for TCHashMap<K, V>
where
    K: PartialEq + Send + Clone + Hash + Sized,
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
            assert_eq!(i + 9, map.get_cloned(&i).expect("none"));
        });
    }
}

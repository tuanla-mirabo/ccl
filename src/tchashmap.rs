//! tchashmap is a highly experimental hashmap based of transactional memory

use swym::thread_key;
use swym::tcell::{TCell, Ref};
use std::hash::Hash;
use std::marker::PhantomData;
// use swym::{ReadTx, RWTx};
use swym::tx::{Borrow, Ordering};

pub const HASHMAP_INIT_CAPACITY_LOOKUP_BITS: usize = 8;

#[inline]
fn init_storage<V: Send + Clone>(capacity: usize) ->  Vec<TCell<Entry<V>>> {
    (0..capacity).map(|_| TCell::new(Entry::Vacant)).collect()
}

#[inline]
fn compute_index(hash: u32, lookup_bits_count: usize) -> usize {
    let shift = 32 - lookup_bits_count;
    (hash >> shift) as usize
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum Entry<V: Send + Sized + Clone> {
    Vacant,
    Occupied(V),
}

unsafe impl<V: Send + Sized + Clone> Borrow for Entry<V> {}

pub struct TCHashMap<K, V>
where
    K: Hash + ?Sized,
    V: Send + Clone,
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
    #[inline]
    pub fn new() -> Self {
        let capacity = 2_usize.pow(HASHMAP_INIT_CAPACITY_LOOKUP_BITS as u32);

        Self {
            storage: init_storage::<V>(capacity),
            lookup_bits_count: HASHMAP_INIT_CAPACITY_LOOKUP_BITS,
            capacity,
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn insert(&self, k: &K, v: V) {
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, self.lookup_bits_count);

        thread_key.rw(|tx| {
            self.storage[index].set(tx, Entry::Occupied(v.clone()))?;
            Ok(())
        });
    }

    #[inline]
    pub fn get_cloned(&self, k: &K) -> Entry<V> {
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, self.lookup_bits_count);
        let mut v = None;

        thread_key.read(|tx| {
            let inner: Result<Ref<Entry<V>>, ()> = Ok(self.storage[index].borrow(tx, Ordering::Read)?);
            if let Ok(inner) = inner {
                v = Some(inner.clone());
            }

            Ok(())
        });

        v.expect("undefined error from swym")
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

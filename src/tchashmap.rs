//! tchashmap is a highly experimental hashmap based of transactional memory

use swym::thread_key;
use swym::tcell::{TCell, Ref};
use std::hash::Hash;
use std::marker::PhantomData;
// use swym::{ReadTx, RWTx};
use swym::tx::{Borrow, Ordering};
use std::sync::atomic;
use parking_lot::RwLock;
use std::mem;

pub const HASHMAP_INIT_CAPACITY_LOOKUP_BITS: usize = 4;
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

pub struct TCHashMap<K, V>
where
    K: Send + Clone + Hash + ?Sized,
    V: Send + Clone,
{
    global_lock: atomic::AtomicBool,
    storage: RwLock<Vec<TCell<Entry<K, V>>>>,
    len: atomic::AtomicUsize,
    lookup_bits_count: atomic::AtomicUsize,
    capacity: atomic::AtomicUsize,
    phantom: PhantomData<K>,
}

impl<K: 'static, V> TCHashMap<K, V>
where
    K: Send+ Clone + Hash + ?Sized,
    V: Send + 'static + Clone,
{
    #[inline]
    pub fn new() -> Self {
        let capacity = 2_usize.pow(HASHMAP_INIT_CAPACITY_LOOKUP_BITS as u32);

        Self {
            global_lock: atomic::AtomicBool::new(false),
            storage: RwLock::new(init_storage::<K, V>(capacity)),
            len: atomic::AtomicUsize::new(0),
            lookup_bits_count: atomic::AtomicUsize::new(HASHMAP_INIT_CAPACITY_LOOKUP_BITS),
            capacity: atomic::AtomicUsize::new(capacity),
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn resize(&self, lbc: usize) {
        while self.global_lock.load(atomic::Ordering::Relaxed) {}

        self.global_lock.store(true, atomic::Ordering::SeqCst);
        let mut storage = self.storage.write();
        self.capacity.fetch_update(|mut x| {
            x *= 2;
            Some(x)
        }, atomic::Ordering::SeqCst, atomic::Ordering::SeqCst).unwrap();
        self.lookup_bits_count.fetch_add(1, atomic::Ordering::SeqCst);
        {
            let old_storage = mem::replace(&mut *storage, init_storage(2_usize.pow(lbc as u32)));
            old_storage.into_iter().for_each(|cell| {
                if let Entry::Occupied(k, v) = cell.into_inner() {
                    let hash = fxhash::hash32(&k);
                    let index = compute_index(hash, lbc);
                    storage[index] = TCell::new(Entry::Occupied(k, v));
                }
            });
        }
        self.global_lock.store(false, atomic::Ordering::SeqCst);
    }

    #[inline]
    pub fn insert(&self, k: K, v: V) {
        while self.global_lock.load(atomic::Ordering::Relaxed) {}

        let len = self.len.load(atomic::Ordering::Relaxed);
        let capacity = self.len.load(atomic::Ordering::Relaxed);
        let lbc = self.lookup_bits_count.load(atomic::Ordering::Relaxed);
        if len + 1 > (capacity as f32 * HASHMAP_LOAD_THRESHOLD) as usize {
            self.resize(lbc + 1);
        }

        self.len.fetch_add(1, atomic::Ordering::Relaxed);
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, self.lookup_bits_count.load(atomic::Ordering::Relaxed));
        let storage = self.storage.read();

        thread_key.rw(|tx| {
            storage[index].set(tx, Entry::Occupied(k.clone(), v.clone()))?;
            Ok(())
        });
    }

    #[inline]
    pub fn get_cloned(&self, k: &K) -> Entry<K, V> {
        let thread_key = thread_key::get();
        let hash = fxhash::hash32(&k);
        let index = compute_index(hash, self.lookup_bits_count.load(atomic::Ordering::Relaxed));
        let mut v = None;
        let storage = self.storage.read();

        thread_key.read(|tx| {
            let inner: Result<Ref<Entry<K, V>>, ()> = Ok(storage[index].borrow(tx, Ordering::Read)?);
            if let Ok(inner) = inner {
                v = Some(inner.clone());
            }

            Ok(())
        });

        v.expect("undefined error from swym")
    }
}

impl<K: 'static, V> Default for TCHashMap<K, V>
where
    K: Send + Clone + Hash + ?Sized,
    V: Send + 'static + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

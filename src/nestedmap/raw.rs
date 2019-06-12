use crate::util;
use crossbeam_epoch::{self as epoch, Atomic, Owned, Pointer, Shared};
use rand::prelude::*;
use std::hash::Hash;
use std::mem;
use std::ops::Deref;
use std::sync::atomic::{Ordering};

const TABLE_SIZE: usize = 32;

#[inline]
fn sharedptr_null<'a, T>() -> Shared<'a, T> {
    unsafe { Shared::from_usize(0) }
}

pub struct Entry<K: Hash + Eq, V> {
    pub key: K,
    pub value: V,
}

pub enum Bucket<K: Hash + Eq, V> {
    Leaf(Entry<K, V>),
    Branch(Table<K, V>),
}

impl<K: Hash + Eq, V> Bucket<K, V> {
    #[inline]
    fn key_ref(&self) -> &K {
        if let Bucket::Leaf(entry) = self {
            &entry.key
        } else {
            panic!("bucket unvalid key get")
        }
    }
}

pub struct Table<K: Hash + Eq, V> {
    nonce: u64,
    buckets: Box<[Atomic<Bucket<K, V>>; TABLE_SIZE]>,
}

pub struct TableRef<'a, K: Hash + Eq, V> {
    guard: Option<epoch::Guard>,
    ptr: &'a Entry<K, V>,
}

impl<'a, K: Hash + Eq, V> Drop for TableRef<'a, K, V> {
    #[inline]
    fn drop(&mut self) {
        let guard = self.guard.take();
        mem::drop(guard);
    }
}

impl<'a, K: Hash + Eq, V> TableRef<'a, K, V> {
    #[inline]
    pub fn key(&self) -> &K {
        &self.ptr.key
    }

    #[inline]
    pub fn value(&self) -> &V {
        &self.ptr.value
    }
}

impl<'a, K: Hash + Eq, V> Deref for TableRef<'a, K, V> {
    type Target = V;

    #[inline]
    fn deref(&self) -> &V {
        &self.value()
    }
}

impl<K: Hash + Eq, V> Drop for Table<K, V> {
    fn drop(&mut self) {
        let guard = &epoch::pin();
        self.buckets.iter().for_each(|ptr| {
            let shared = ptr.load(Ordering::SeqCst, guard);

            if !shared.is_null() {
                unsafe { guard.defer_destroy(shared) };
            }
        });
    }
}

impl<'a, K: 'a + Hash + Eq, V: 'a> Table<K, V> {
    #[inline]
    fn with_two_entries(
        entry_1: Shared<'a, Bucket<K, V>>,
        entry_2: Shared<'a, Bucket<K, V>>,
    ) -> Self {
        unsafe { debug_assert!(entry_1.as_ref().unwrap().key_ref() != entry_2.as_ref().unwrap().key_ref(), "table dual new keys equal"); }

        let mut table = Self::empty();
        let entry_1_pos = unsafe {
            util::hash_with_nonce(entry_1.as_ref().unwrap().key_ref(), table.nonce) as usize
                % TABLE_SIZE
        };
        let entry_2_pos = unsafe {
            util::hash_with_nonce(entry_2.as_ref().unwrap().key_ref(), table.nonce) as usize
                % TABLE_SIZE
        };

        if entry_1_pos != entry_2_pos {
            table.buckets[entry_1_pos].store(entry_1, Ordering::SeqCst);
            table.buckets[entry_2_pos].store(entry_2, Ordering::SeqCst);
        } else {
            table.buckets[entry_1_pos] =
                Atomic::new(Bucket::Branch(Table::with_two_entries(entry_1, entry_2)));
        }

        table
    }

    #[inline]
    pub fn empty() -> Self {
        Self {
            nonce: rand::thread_rng().gen(),
            buckets: unsafe { Box::new(mem::zeroed()) },
        }
    }

    #[inline]
    pub fn get(&'a self, key: &K) -> Option<TableRef<'a, K, V>> {
        let guard = epoch::pin();
        let fake_guard = unsafe { epoch::unprotected() };
        let key_pos = util::hash_with_nonce(key, self.nonce) as usize % TABLE_SIZE;

        let bucket_shared: Shared<'a, Bucket<K, V>> =
            self.buckets[key_pos].load(Ordering::SeqCst, fake_guard);

        if bucket_shared.is_null() {
            None
        } else {
            let bucket_ref = unsafe { bucket_shared.deref() };

            match bucket_ref {
                Bucket::Leaf(entry) => {
                    if &entry.key == key {
                        Some(TableRef {
                            guard: Some(guard),
                            ptr: entry,
                        })
                    } else {
                        None
                    }
                }

                Bucket::Branch(table) => table.get(key),
            }
        }
    }

    #[inline]
    pub fn insert(&self, entry: Owned<Bucket<K, V>>) {
        let guard = &epoch::pin();
        let key_pos = util::hash_with_nonce(entry.key_ref(), self.nonce) as usize % TABLE_SIZE;
        let bucket = &self.buckets[key_pos];

        let mut entry = Some(entry);

        match bucket.compare_and_set(
            sharedptr_null(),
            entry.take().unwrap(),
            Ordering::SeqCst,
            guard,
        ) {
            Ok(_) => {}

            Err(err) => {
                entry = Some(err.new);
                let actual = err.current;
                let actual_ref = unsafe { actual.as_ref().expect("insert1 null") };

                let entry = entry.take().unwrap();
                match actual_ref {
                    Bucket::Branch(ref table) => table.insert(entry),
                    Bucket::Leaf(ref old_entry) => {
                        if entry.key_ref() == &old_entry.key {
                            bucket.store(entry, Ordering::SeqCst);
                            unsafe { guard.defer_destroy(actual) }
                        } else {
                            let new_table = Owned::new(Bucket::Branch(Table::with_two_entries(
                                actual,
                                entry.into_shared(guard),
                            )));
                            bucket.store(new_table, Ordering::SeqCst);
                        }
                    }
                }
            }
        }
    }

    #[inline]
    pub fn remove(&self, key: &K) {
        let guard = &epoch::pin();
        let key_pos = util::hash_with_nonce(key, self.nonce) as usize % TABLE_SIZE;

        let bucket_sharedptr = self.buckets[key_pos].load(Ordering::SeqCst, guard);

        if let Some(bucket_ref) = unsafe { bucket_sharedptr.as_ref() } {
            match bucket_ref {
                Bucket::Branch(table) => table.remove(key),
                Bucket::Leaf(_) => {
                    let res = self.buckets[key_pos].compare_and_set(bucket_sharedptr, sharedptr_null(), Ordering::SeqCst, guard);

                    if res.is_ok() {
                        unsafe { guard.defer_destroy(bucket_sharedptr) };
                    }
                }
            }
        }
    }
}

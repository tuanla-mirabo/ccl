use crate::util;
use std::mem;
use crossbeam_epoch::{self as epoch, Atomic, Guard, Owned, Pointer, Shared};
use std::hash::Hash;
use std::sync::atomic::{spin_loop_hint as cpu_relax, Ordering};
use rand::prelude::*;
use std::ops::Deref;

const TABLE_SIZE: usize = 256;

struct Entry<K: Hash + Eq, V> {
    key: K,
    value: V,
}

enum Bucket<K: Hash + Eq, V> {
    Leaf(Entry<K, V>),
    Branch(Table<K, V>),
}

struct Table<K: Hash + Eq, V> {
    nonce: u64,
    buckets: [Atomic<Bucket<K, V>>; TABLE_SIZE],
}

struct TableRef<'a, V> {
    guard: Option<epoch::Guard>,
    ptr: &'a V,
}

impl<'a, V> Drop for TableRef<'a, V> {
    fn drop(&mut self) {
        let guard = self.guard.take();
        mem::drop(guard);
    }
}

impl<'a, V> Deref for TableRef<'a, V> {
    type Target = V;

    fn deref(&self) -> &V {
        self.ptr
    }
}

impl<'a, K: 'a + Hash + Eq, V: 'a> Table<K, V> {
    fn with_two_entries(entry_1: Entry<K, V>, entry_2: Entry<K, V>) -> Self {
        let mut table = Self::empty();
        let entry_1_pos = util::hash_with_nonce(&entry_1.key, table.nonce) as usize % TABLE_SIZE;
        let entry_2_pos = util::hash_with_nonce(&entry_1.key, table.nonce) as usize % TABLE_SIZE;

        if entry_1_pos != entry_2_pos {
            table.buckets[entry_1_pos] = Atomic::new(Bucket::Leaf(entry_1));
            table.buckets[entry_2_pos] = Atomic::new(Bucket::Leaf(entry_2));
        } else {
            table.buckets[entry_1_pos] = Atomic::new(Bucket::Branch(Table::with_two_entries(entry_1, entry_2)));
        }

        table
    }

    fn empty() -> Self {
        Self {
            nonce: rand::thread_rng().gen(),
            buckets: unsafe { mem::zeroed() },
        }
    }

    fn get(&'a self, key: &K) -> Option<TableRef<'a, V>> {
        let guard = epoch::pin();
        let fake_guard = unsafe { epoch::unprotected() };
        let key_pos = util::hash_with_nonce(key, self.nonce) as usize % TABLE_SIZE;

        let bucket_shared: Shared<'a, Bucket<K, V>> = self.buckets[key_pos].load(Ordering::SeqCst, fake_guard);

        if bucket_shared.is_null() {
            None
        } else {
            let bucket_ref = unsafe { bucket_shared.deref() };

            match bucket_ref {
                Bucket::Leaf(entry) => {
                    Some(TableRef {
                        guard: Some(guard),
                        ptr: &entry.value,
                    })
                }

                Bucket::Branch(table) => {
                    table.get(key)
                }
            }
        }
    }
}

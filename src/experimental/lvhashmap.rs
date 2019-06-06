use std::hash::Hash;
use std::sync::atomic::{Ordering, AtomicBool, AtomicUsize, spin_loop_hint as cpu_relax};
use crossbeam_epoch::{self as epoch, Atomic, Owned, Shared, Guard};

const USIZE_MSB: usize = std::isize::MIN as usize;

struct Entry<K: Hash + Eq, V> {
    key: K,
    value: V,
    key_hash: u64,
    lock: AtomicUsize,
}

impl<K: Hash + Eq, V> Entry<K, V> {
    fn aquire_read(&self) {
        while {
            let mut old;

            while {
                old = self.lock.load(Ordering::Relaxed);
                old & USIZE_MSB != 0
            } {
                cpu_relax();
            }

            old &= !USIZE_MSB;

            let new = old + 1;
            debug_assert!(new != (!USIZE_MSB) & (!0));

            self.lock.compare_and_swap(old, new, Ordering::SeqCst) != old
        } {
            cpu_relax();
        }
    }

    fn release_read(&self) {
        debug_assert!(self.lock.load(Ordering::Relaxed) & (!USIZE_MSB) > 0);
        self.lock.fetch_sub(1, Ordering::SeqCst);
    }

    fn aquire_write(&self) {
        loop
        {
            let old = (!USIZE_MSB) & self.lock.load(Ordering::Relaxed);
            let new = USIZE_MSB | old;
            if self.lock.compare_and_swap(old,
                                          new,
                                          Ordering::SeqCst) == old
            {
                while self.lock.load(Ordering::Relaxed) != USIZE_MSB {
                    cpu_relax();
                }
                break
            }
        }
    }

    fn release_write(&self) {
        debug_assert_eq!(self.lock.load(Ordering::Relaxed), USIZE_MSB);
        self.lock.store(0, Ordering::Relaxed);
    }
}

enum Bucket<K: Hash + Eq, V> {
    Empty,
    Tombstone,
    Occupied(Entry<K, V>),
}

struct Table<K: Hash + Eq, V> {
    resize_in_progress: AtomicBool,
    resize_ready: AtomicBool,
    resize_new_table: Atomic<Table<K, V>>,
    load_factor_ctr: AtomicUsize,
    data: Box<[Bucket<K, V>]>,
}

use crate::util;
use crossbeam_epoch::{self as epoch, Atomic, Guard, Owned, Pointer, Shared};
use std::hash::Hash;
use std::sync::atomic::{spin_loop_hint as cpu_relax, AtomicBool, AtomicUsize, Ordering};

const USIZE_MSB: usize = std::isize::MIN as usize;
const LOAD_FACTOR_MAX: f64 = 0.75;
static REDIRECT_BUCKET: Bucket<i32, i32> = Bucket::Redirect;

fn null_owned<T>() -> Owned<T> {
    unsafe { Owned::from_usize(0) }
}

trait ExtendedPointer<T> {
    unsafe fn as_ref_extptr(&self) -> &T;
}

impl<'g, T> ExtendedPointer<T> for Shared<'g,  T> {
    unsafe fn as_ref_extptr(&self) -> &T {
        self.as_ref().unwrap()
    }
}

impl<T> ExtendedPointer<T> for Owned<T> {
    unsafe fn as_ref_extptr(&self) -> &T {
        &*self
    }
}

fn make_redirect_static() -> usize {
    let rptr = &REDIRECT_BUCKET as *const Bucket<i32, i32>;
    rptr as usize
}

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
        loop {
            let old = (!USIZE_MSB) & self.lock.load(Ordering::Relaxed);
            let new = USIZE_MSB | old;
            if self.lock.compare_and_swap(old, new, Ordering::SeqCst) == old {
                while self.lock.load(Ordering::Relaxed) != USIZE_MSB {
                    cpu_relax();
                }
                break;
            }
        }
    }

    fn release_write(&self) {
        debug_assert_eq!(self.lock.load(Ordering::Relaxed), USIZE_MSB);
        self.lock.store(0, Ordering::Relaxed);
    }
}

enum Bucket<K: Hash + Eq, V> {
    Tombstone,
    Redirect,
    Occupied(Entry<K, V>),
}

struct Table<K: Hash + Eq, V> {
    resize_in_progress: AtomicBool,
    resize_ready: AtomicBool,
    resize_new_table: Atomic<Table<K, V>>,
    load_factor_ctr: AtomicUsize,
    data: Box<[Atomic<Bucket<K, V>>]>,
}

impl<'a, K: 'a + Hash + Eq, V: 'a> Table<K, V> {
    fn new(capacity: usize) -> Self {
        let capacity = util::round_pow2(capacity);
        let lfctr = (capacity as f64 * LOAD_FACTOR_MAX) as usize;
        let data = (0..capacity)
            .map(|_| Atomic::null())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            resize_in_progress: AtomicBool::new(false),
            resize_ready: AtomicBool::new(false),
            resize_new_table: Atomic::null(),
            load_factor_ctr: AtomicUsize::new(lfctr),
            data,
        }
    }

    fn insert_ptr_with_hash<P: Pointer<Bucket<K, V>> + ExtendedPointer<Bucket<K, V>>>(&self, hash: u64, ptr: P) {
        let guard = &epoch::pin();
        let idx = hash as usize % self.data.len();
        let mut probe_offset = 0;

        let key = if let Bucket::Occupied(entry) = unsafe { ptr.as_ref_extptr() } {
            &entry.key
        } else {
            panic!("what is even");
        };

        loop {
            let slot = &self.data[idx + probe_offset];
            let slot_bucket_ptr = slot.load(Ordering::SeqCst, guard);
            if slot_bucket_ptr.is_null() {
                slot.store(ptr, Ordering::SeqCst);
                break;
            } else {
                let slot_ref = unsafe { slot_bucket_ptr.as_ref_extptr() };
                match slot_ref {
                    Bucket::Tombstone => {}
                    Bucket::Redirect => {}
                    Bucket::Occupied(entry) => {}
                }
            }
        }
    }

    fn resize(&self, new_capacity: usize, selfptr: &Atomic<Table<K, V>>) {
        let guard = &epoch::pin();
        
        if self
            .resize_in_progress
            .compare_and_swap(false, true, Ordering::SeqCst)
        {
            // resize flag already set, probably in progress
            return;
        }

        let new_table = Self::new(new_capacity);

        // publish new table internally
        if !self
            .resize_new_table
            .swap(Owned::new(new_table), Ordering::SeqCst, guard)
            .is_null()
        {
            panic!("old resize_new_table ptr was not null, something has gone very wrong");
        }

        self.resize_ready.store(true, Ordering::SeqCst);

        let new_table_ref = unsafe {
            self.resize_new_table
                .load(Ordering::SeqCst, guard)
                .as_ref()
                .unwrap()
        };

        for ptr in self.data.iter() {
            let shared = ptr.load(Ordering::Relaxed, guard);
            if let Some(bucket) = unsafe { shared.as_ref() } {
                if let Bucket::Occupied(entry) = bucket {
                    new_table_ref.insert_ptr_with_hash(entry.key_hash, shared);
                    let redirect_ptr: Shared<Bucket<K, V>> =
                        unsafe { Shared::from_usize(make_redirect_static()) };
                    ptr.store(redirect_ptr, Ordering::SeqCst);
                }
            }
        }

        // publish table
        let current_table = selfptr.swap(self.resize_new_table.load(Ordering::SeqCst, guard), Ordering::SeqCst, guard);
        unsafe { guard.defer_destroy(current_table); }

        self.resize_in_progress.store(false, Ordering::SeqCst);
        self.resize_ready.store(false, Ordering::SeqCst);

        // remove new_table_ptr
        self.resize_new_table.store(null_owned(), Ordering::SeqCst);
    }
}

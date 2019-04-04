use std::sync::Arc;
use std::hash::Hash;
use crate::dhashmap::DHashMap;
use parking_lot::Mutex;
use std::time::{Instant, Duration};
use std::future::Future;
use std::pin::Pin;

pub const VALID_DURATION: Duration = Duration::from_secs(3 * 60 * 60);
pub const VALID_CHECK_INTERVAL: Duration = Duration::from_secs(15 * 60);
pub const SAVE_INTERVAL: Duration = Duration::from_secs(3 * 60);

struct Entry<V> {
    time: Instant,
    saved: bool,
    data: V,
}

impl<V> Entry<V> {
    fn new(data: V) -> Self {
        Self {
            time: Instant::now(),
            saved: true,
            data,
        }
    }

    fn set_saved(&mut self, saved: bool) {
        self.saved = saved;
    }

    fn to_evict(&self, now: Instant) -> bool {
        self.time.duration_since(now) > VALID_DURATION && self.saved
    }

    fn get(&self) -> &V {
        &self.data
    }

    fn get_mut(&mut self) -> &mut V {
        &mut self.data
    }
}

pub struct HLTimedCache<K, V>
where
    K: Hash + Eq + Clone
{
    pub inner: Arc<HLTimedCacheInner<K, V>>,
}

pub struct HLTimedCacheInner<K, V>
where
    K: Hash + Eq + Clone
{
    // stores saved values
    saved: DHashMap<u64, Entry<V>>,

    // stores unsaved values
    unsaved: DHashMap<u64, Entry<V>>,

    // stores a bool, if true, the value is saved, if false, the value is unsaved
    lookup: DHashMap<K, bool>,

    // timestamp of last save
    last_saved: Mutex<Instant>,

    // timestamp of last purge
    last_purged: Mutex<Instant>,

    // item load function
    load_item_fn: fn(&K) -> Option<V>,

    // item save function
    save_item_fn: fn(&K, &V) -> Pin<Box<Future<Output = bool> + Send>>,
}

impl<K, V> HLTimedCacheInner<K, V>
where
    K: Hash + Eq + Clone
{
    pub fn new(load_item: fn(&K) -> Option<V>, save_item: fn(&K, &V) -> Pin<Box<Future<Output = bool> + Send>>) -> Self {
        Self {
            saved: DHashMap::new(),
            unsaved: DHashMap::new(),
            lookup: DHashMap::new(),
            last_saved: Mutex::new(Instant::now()),
            last_purged: Mutex::new(Instant::now()),
            load_item_fn: load_item,
            save_item_fn: save_item,
        }
    }

    pub fn load_item(&self, key: &K) {
        if !self.lookup.contains_key(key) {
            if let Some(v) = (self.load_item_fn)(key) {
                let v = Entry::new(v);
                let h = fxhash::hash64(key);
                self.lookup.insert(key.clone(), true);
                self.saved.insert(h, v);
            }
        }
    }

    pub fn map<T, F: FnOnce(&V) -> T>(&self, key: &K, f: F) -> T {
        self.load_item(key);
        let s = self.lookup.get(key).unwrap();
        let h = fxhash::hash64(key);
        let data = if *s {
            self.saved.get(&h).unwrap()
        } else {
            self.unsaved.get(&h).unwrap()
        };
        f(data.get())
    }

    pub fn map_mut<T, F: FnOnce(&mut V) -> T>(&self, key: &K, mut f: F) -> T {
        self.load_item(key);
        let s = self.lookup.get(key).unwrap();
        let h = fxhash::hash64(key);
        let mut data = if *s {
            self.saved.get_mut(&h).unwrap()
        } else {
            self.unsaved.get_mut(&h).unwrap()
        };
        f(data.get_mut())
    }
}

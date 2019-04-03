//! threadsafe concurrent timed cache based of dhashmap

use crate::dhashmap::DHashMap;
use parking_lot::Mutex;
use std::hash::Hash;
use std::time;

pub const VALID_DURATION: time::Duration = time::Duration::from_secs(6 * 60 * 60);
pub const VALID_CHECK_INTERVAL: time::Duration = time::Duration::from_secs(30 * 60);
pub const SAVE_INTERVAL: time::Duration = time::Duration::from_secs(3 * 60);

pub struct TimedCache<K, V>
where
    K: Hash + Eq + Clone,
{
    storage: DHashMap<K, (V, time::Instant, bool)>,
    load_item_fn: fn(&K) -> Option<V>,
    save_item_fn: fn(&K, &V) -> bool,
    last_saved: Mutex<time::Instant>,
    last_purged: Mutex<time::Instant>,
}

impl<'a, K: Hash + Eq + Clone, V> TimedCache<K, V> {
    pub fn new(load_item: fn(&K) -> Option<V>, save_item: fn(&K, &V) -> bool) -> Self {
        Self {
            storage: DHashMap::new(),
            load_item_fn: load_item,
            save_item_fn: save_item,
            last_saved: Mutex::new(time::Instant::now()),
            last_purged: Mutex::new(time::Instant::now()),
        }
    }

    pub fn load_item(&self, k: &K) {
        if !self.storage.contains_key(k) {
            if let Some(v) = (self.load_item_fn)(k) {
                let v = (v, time::Instant::now(), true);
                self.storage.insert(k.clone(), v);
            }
        }
    }

    pub fn map<T, F: FnOnce(&V) -> T>(&self, k: &K, f: F) -> T {
        self.load_item(k);
        let data = self.storage.get(k).unwrap();
        f(&data.0)
    }

    pub fn map_mut<T, F: FnMut(&mut V) -> T>(&self, k: &K, mut f: F) -> T {
        self.load_item(k);
        let mut data = self.storage.get_mut(k).unwrap();
        data.2 = false;
        f(&mut data.0)
    }

    pub fn do_check(&self) {
        let now = time::Instant::now();
        let mut last_saved = self.last_saved.lock();
        let mut last_purged = self.last_purged.lock();

        let check_save_item = |k: &K, v: &mut (V, time::Instant, bool)| {
            if !v.2 && (self.save_item_fn)(k, &v.0) {
                v.2 = true;
            }
        };

        let check_to_evict = |_k: &K, v: &mut (V, time::Instant, bool)| -> bool {
            now.duration_since(v.1) > VALID_DURATION && v.2
        };

        if now.duration_since(*last_saved) > SAVE_INTERVAL {
            *last_saved = now;

            self.storage.submaps_write().for_each(|mut submap| {
                submap
                    .iter_mut()
                    .for_each(|(k, mut v)| check_save_item(&k, &mut v))
            });
        }

        if now.duration_since(*last_purged) > VALID_CHECK_INTERVAL {
            *last_purged = now;

            self.storage.retain(!check_to_evict);
        }
    }
}

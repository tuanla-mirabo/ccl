//! Threadsafe concurrent timed cache based of DHashMap.
//! Handles loading and potential saving behind the scenes with user supplied functions.

use crate::dhashmap::DHashMap;
use parking_lot::Mutex;
use std::hash::Hash;
use std::time;

pub const VALID_DURATION: time::Duration = time::Duration::from_secs(6 * 60 * 60);
pub const VALID_CHECK_INTERVAL: time::Duration = time::Duration::from_secs(30 * 60);
pub const SAVE_INTERVAL: time::Duration = time::Duration::from_secs(3 * 60);

/// Threadsafe concurrent timed cache based of DHashMap.
/// Handles loading and potential saving behind the scenes with user supplied functions.
pub struct TimedCache<K, V>
where
    K: Hash + Eq + Clone,
{
    storage: DHashMap<K, (V, time::Instant, bool)>,
    load_item_fn: fn(&K) -> Option<V>,
    save_item_fn: fn(&K, &V) -> bool,
    last_saved: Mutex<time::Instant>,
    last_purged: Mutex<time::Instant>,
    valid_duration: time::Duration,
    valid_check_interval: time::Duration,
    save_interval: time::Duration,
}

impl<'a, K: Hash + Eq + Clone, V> TimedCache<K, V> {
    /// Creates a new TimedCache. Saving function may be empty if no custom saving functionality is needed.
    /// Takes three duration arguments. Supply `None` to use the defaults.
    pub fn new(load_item: fn(&K) -> Option<V>, save_item: fn(&K, &V) -> bool, valid_duration: Option<time::Duration>, valid_check_interval: Option<time::Duration>, save_interval: Option<time::Duration>) -> Self {
        Self {
            storage: DHashMap::new(),
            load_item_fn: load_item,
            save_item_fn: save_item,
            last_saved: Mutex::new(time::Instant::now()),
            last_purged: Mutex::new(time::Instant::now()),
            valid_duration: valid_duration.unwrap_or(VALID_DURATION),
            valid_check_interval: valid_check_interval.unwrap_or(VALID_CHECK_INTERVAL),
            save_interval: save_interval.unwrap_or(SAVE_INTERVAL),
        }
    }

    /// Load an item with a specified key. Intended to mainly be called from `map` and `map_mut`
    pub fn load_item(&self, k: &K) {
        if !self.storage.contains_key(k) {
            if let Some(v) = (self.load_item_fn)(k) {
                let v = (v, time::Instant::now(), true);
                self.storage.insert(k.clone(), v);
            }
        }
    }

    /// Takes a closure with a normal reference as an argument and executes it.
    /// The function will return the same value as the closure which means the function can be used to extract data.
    pub fn map<T, F: FnOnce(&V) -> T>(&self, k: &K, f: F) -> T {
        self.load_item(k);
        let data = self.storage.get(k).unwrap();
        f(&data.0)
    }

    /// Takes a closure with a mutable reference as an argument and executes it.
    /// The function will return the same value as the closure which means the function can be used to extract data.
    pub fn map_mut<T, F: FnMut(&mut V) -> T>(&self, k: &K, mut f: F) -> T {
        self.load_item(k);
        let mut data = self.storage.get_mut(k).unwrap();
        data.2 = false;
        f(&mut data.0)
    }

    /// Performance maintenance tasks like saving and evicting invalid entries.
    /// May take significant time depending on amount of entries and the time complexity of saving each.
    /// This is intended to be improved in a future iteration of TimedCache.
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
            now.duration_since(v.1) > self.valid_duration && v.2
        };

        if now.duration_since(*last_saved) > self.save_interval {
            *last_saved = now;

            self.storage.submaps_write().for_each(|mut submap| {
                submap
                    .iter_mut()
                    .for_each(|(k, mut v)| check_save_item(&k, &mut v))
            });
        }

        if now.duration_since(*last_purged) > self.valid_check_interval {
            *last_purged = now;

            self.storage.retain(|k, v| !check_to_evict(k, v));
        }
    }
}

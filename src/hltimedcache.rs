use std::hash::Hash;
use crate::dhashmap::DHashMap;
use parking_lot::Mutex;
use std::time::{Instant, Duration};
use hashbrown::HashMap;
use std::mem;
use std::sync::Arc;

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

impl<K, V> HLTimedCache<K, V>
where
    K: Hash + Eq + Clone
{
    pub fn new(load_item: fn(&K) -> Option<V>, save_item: fn(&K, &V) -> bool) -> Self {
        Self {
            inner: Arc::new(HLTimedCacheInner::new(load_item, save_item)),
        }
    }

    pub fn map<T, F: FnOnce(&V) -> T>(&self, key: &K, f: F) -> T {
        self.inner.map(key, f)
    }

    pub fn map_mut<T, F: FnOnce(&mut V) -> T>(&self, key: &K, f: F) -> T {
        self.inner.map_mut(key, f)
    }

    pub fn do_check(&self) {
        self.inner.do_check();
    }
}

pub struct HLTimedCacheInner<K, V>
where
    K: Hash + Eq + Clone
{
    // stores saved values
    saved: DHashMap<K, Entry<V>>,

    // stores unsaved values
    unsaved: DHashMap<K, Entry<V>>,

    // stores a bool, if true, the value is saved, if false, the value is unsaved
    lookup: DHashMap<K, bool>,

    // timestamp of last save
    last_saved: Mutex<Instant>,

    // timestamp of last purge
    last_purged: Mutex<Instant>,

    // item load function
    load_item_fn: fn(&K) -> Option<V>,

    // item save function
    save_item_fn: fn(&K, &V) -> bool,
}

impl<K, V> HLTimedCacheInner<K, V>
where
    K: Hash + Eq + Clone,
{
    pub fn new(load_item: fn(&K) -> Option<V>, save_item: fn(&K, &V) -> bool) -> Self {
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
                self.lookup.insert(key.clone(), true);
                self.saved.insert(key.clone(), v);
            }
        }
    }

    pub fn map<T, F: FnOnce(&V) -> T>(&self, key: &K, f: F) -> T {
        self.load_item(key);
        let s = self.lookup.get(key).unwrap();
        let data = if *s {
            self.saved.get(key).unwrap()
        } else {
            self.unsaved.get(key).unwrap()
        };
        f(data.get())
    }

    pub fn map_mut<T, F: FnOnce(&mut V) -> T>(&self, key: &K, f: F) -> T {
        self.load_item(key);
        let mut s = self.lookup.get_mut(key).unwrap();
        if *s {
            let mut e = self.saved.remove(key).unwrap();
            e.1.set_saved(false);
            self.unsaved.insert(e.0, e.1);
            *s = false;
        }
        let mut data = self.unsaved.get_mut(key).unwrap();
        f(data.get_mut())
    }

    pub fn do_check(&self) {
        let now = Instant::now();
        let mut last_saved = self.last_saved.lock();
        let mut last_purged = self.last_purged.lock();

        if now.duration_since(*last_saved) > SAVE_INTERVAL {
            *last_saved = now;

            self.unsaved.submaps_write().for_each(|mut submap| {
                let old_submap = mem::replace(&mut *submap, HashMap::new());

                old_submap.into_iter().for_each(|(k, mut v)| {
                    let save_status = (self.save_item_fn)(&k, v.get());
                    if save_status {
                        *self.lookup.get_mut(&k).unwrap() = true;
                        v.set_saved(true);
                        self.saved.insert(k, v);
                    }
                });
            });
        }

        if now.duration_since(*last_purged) > VALID_CHECK_INTERVAL {
            *last_purged = now;

            self.saved.retain(|_k, v| !v.to_evict(now));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_map_mut() {
        fn load(key: &u64) -> Option<String> {
            Some(key.to_string())
        }

        fn save(_key: &u64, _value: &String) -> bool { true }

        let cache = HLTimedCache::new(load, save);

        cache.map_mut(&1919, |v| v.clear());
    }
}

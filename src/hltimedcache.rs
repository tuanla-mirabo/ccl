use std::sync::Arc;
use std::hash::Hash;
use crate::dhashmap::DHashMap;
use parking_lot::Mutex;
use std::time::{Instant, Duration};
use std::future::Future;
use std::pin::Pin;
use hashbrown::HashMap;
use std::mem;
use tokio::await;

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
    saved: Arc<DHashMap<K, Entry<V>>>,

    // stores unsaved values
    unsaved: Arc<DHashMap<K, Entry<V>>>,

    // stores a bool, if true, the value is saved, if false, the value is unsaved
    lookup: Arc<DHashMap<K, bool>>,

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
    K: 'static + Hash + Eq + Clone + Send + Sync,
    V: 'static + Send + Sync,
{
    pub fn new(load_item: fn(&K) -> Option<V>, save_item: fn(&K, &V) -> Pin<Box<Future<Output = bool> + Send>>) -> Self {
        Self {
            saved: Arc::new(DHashMap::new()),
            unsaved: Arc::new(DHashMap::new()),
            lookup: Arc::new(DHashMap::new()),
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

    pub fn do_check(&self, f: bool) {
        let now = Instant::now();
        let mut last_saved = self.last_saved.lock();
        let mut last_purged = self.last_purged.lock();

        if now.duration_since(*last_saved) > SAVE_INTERVAL || f {
            *last_saved = now;
            let h_unsaved = self.unsaved.clone();
            let h_saved = self.saved.clone();
            let h_lookup = self.lookup.clone();
            let h_save_item_fn = self.save_item_fn;

            fn app_save<K: 'static, V: 'static>(save_item_fn: fn(&K, &V) -> Pin<Box<Future<Output = bool> + Send>>, k: K, mut v: Entry<V>, l: Arc<DHashMap<K, bool>>, s: Arc<DHashMap<K, Entry<V>>>)
            where
                K: Hash + Eq + Clone + Send + Sync,
                V: 'static + Send + Sync,
            {
                tokio::spawn_async(async move {
                    await!(save_item_fn(&k, v.get()));
                    v.set_saved(true);
                    let mut saved_b = l.get_mut(&k).unwrap();
                    *saved_b = true;
                    drop(saved_b);
                    s.insert(k, v);
                });
            }

            let app = async move || {
                h_unsaved.submaps_write().for_each(|mut submap| {
                    if submap.len() > 0 {
                        let cap = submap.capacity();
                        let oldmap = mem::replace(&mut *submap, HashMap::with_capacity(cap));

                        oldmap.into_iter().for_each(|(k, v)| {
                            app_save(h_save_item_fn, k, v, h_lookup.clone(), h_saved.clone());
                        });
                    }
                });
            };

            tokio::run_async(app());
        }

        //if now.duration_since(*last_purged) > VALID_CHECK_INTERVAL {
        //        *last_purged = now;
        //    self.storage.retain(|k, v| !check_to_evict(k, v));
        //}
    }
}

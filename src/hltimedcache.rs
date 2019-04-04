use std::sync::Arc;
use std::hash::Hash;
use crate::dhashmap::DHashMap;
use parking_lot::Mutex;
use std::time::{Instant, Duration};

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
    save_item_fn: fn(&K, &V) -> bool,
}

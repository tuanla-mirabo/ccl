use std::sync::Arc;
use std::hash::Hash;
use crate::dhashmap::DHashMap;
use parking_lot::Mutex;
use std::time;

pub struct HLTimedCache<K, V>
where
    K: Hash + Eq + Clone
{
    inner: Arc<HLTimedCacheInner<K, V>>,
}

pub struct HLTimedCacheInner<K, V>
where
    K: Hash + Eq + Clone
{
    saved: DHashMap<K, V>,
    unsaved: DHashMap<K, V>,
    lookup: DHashMap<K, V>,
}

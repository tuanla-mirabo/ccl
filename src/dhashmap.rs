//! dhashmap is a threadsafe concurrent hashmap with good allround
//! performance which trading memory usage for concurrency
//!
//! the api mostly matches that of the standard library hashmap but there are some
//! differences to due to the design of the hashmap
//!
//! initialization is fairly costly

use hashbrown::HashMap;
use parking_lot::RwLock;
use smallvec::SmallVec;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

// the amount of bits to look at when determining maps
const NCB: u64 = 8;

// number of maps, needs to be 2^NCB
const NCM: usize = 256;

#[derive(Default)]
pub struct DHashMap<K, V>
where
    K: Hash + Eq,
{
    submaps: SmallVec<[RwLock<HashMap<K, V>>; NCM]>,
}

impl<'a, K: 'a, V: 'a> DHashMap<K, V>
where
    K: Hash + Eq,
{
    #[inline]
    pub fn new() -> Self {
        if !check_opt(NCB, NCM) {
            panic!("dhashmap params illegal");
        }

        Self {
            submaps: (0..NCM).map(|_| RwLock::new(HashMap::new())).collect(),
        }
    }

    #[inline]
    pub fn insert(&self, key: K, value: V) {
        let mapi = determine_map(hash(&key));
        let mut submap = self.submaps[mapi].write();
        submap.insert(key, value);
    }

    #[inline]
    pub fn contains_key(&self, key: &K) -> bool {
        let mapi = determine_map(hash(&key));
        let submap = self.submaps[mapi].read();
        submap.contains_key(&key)
    }

    #[inline]
    pub fn get(&'a self, key: &'a K) -> Option<DHashMapRef<'a, K, V>> {
        let mapi = determine_map(hash(&key));
        let submap = self.submaps[mapi].read();
        if submap.contains_key(&key) {
            Some(DHashMapRef { lock: submap, key })
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mut(&'a self, key: &'a K) -> Option<DHashMapRefMut<'a, K, V>> {
        let mapi = determine_map(hash(&key));
        let submap = self.submaps[mapi].write();
        if submap.contains_key(&key) {
            Some(DHashMapRefMut { lock: submap, key })
        } else {
            None
        }
    }

    #[inline]
    pub fn remove(&self, key: &K) {
        let mapi = determine_map(hash(&key));
        let mut submap = self.submaps[mapi].write();
        submap.remove(key);
    }

    #[inline]
    pub fn retain<F: Clone + FnMut(&K, &mut V) -> bool>(&self, f: F) {
        self.submaps.iter().for_each(|locked| {
            let mut submap = locked.write();
            submap.retain(f.clone());
        });
    }

    #[inline]
    pub fn clear(&self) {
        self.submaps.iter().for_each(|locked| {
            let mut submap = locked.write();
            submap.clear();
        });
    }

    #[inline]
    pub fn submaps_read(
        &self,
    ) -> impl Iterator<Item = parking_lot::RwLockReadGuard<HashMap<K, V>>> {
        self.submaps.iter().map(|locked| locked.read())
    }

    #[inline]
    pub fn submaps_write(
        &self,
    ) -> impl Iterator<Item = parking_lot::RwLockWriteGuard<HashMap<K, V>>> {
        self.submaps.iter().map(|locked| locked.write())
    }
}

#[inline]
fn check_opt(ncb: u64, ncm: usize) -> bool {
    2_u64.pow(ncb as u32) == ncm as u64
}

#[inline]
fn hash<V: Hash>(data: &V) -> u64 {
    fxhash::hash64(data)
}

#[inline]
fn determine_map(hash: u64) -> usize {
    let shift = 64 - NCB;
    (hash >> shift) as usize
}

pub struct DHashMapRef<'a, K, V>
where
    K: Hash + Eq,
{
    pub lock: parking_lot::RwLockReadGuard<'a, HashMap<K, V>>,
    pub key: &'a K,
}

impl<'a, K, V> Deref for DHashMapRef<'a, K, V>
where
    K: Hash + Eq,
{
    type Target = V;

    #[inline]
    fn deref(&self) -> &V {
        self.lock.get(self.key).unwrap()
    }
}

pub struct DHashMapRefMut<'a, K, V>
where
    K: Hash + Eq,
{
    pub lock: parking_lot::RwLockWriteGuard<'a, HashMap<K, V>>,
    pub key: &'a K,
}

impl<'a, K, V> Deref for DHashMapRefMut<'a, K, V>
where
    K: Hash + Eq,
{
    type Target = V;

    #[inline]
    fn deref(&self) -> &V {
        self.lock.get(self.key).unwrap()
    }
}

impl<'a, K, V> DerefMut for DHashMapRefMut<'a, K, V>
where
    K: Hash + Eq,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut V {
        self.lock.get_mut(self.key).unwrap()
    }
}

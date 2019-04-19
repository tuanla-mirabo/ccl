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
use std::hash::Hasher;
use std::ops::{Deref, DerefMut};

/// The amount of bits to look at when determining maps.
const NCB: u64 = 8;

// Amount of shards. Equals 2^NCB.
const NCM: usize = 1 << NCB;

#[derive(Default)]
pub struct DHashMap<K, V>
where
    K: Hash + Eq,
{
    submaps: SmallVec<[RwLock<HashMap<K, V>>; NCM]>,
    hash_nonce: u64,
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
            hash_nonce: rand::random(),
        }
    }

    #[inline]
    pub fn insert(&self, key: K, value: V) {
        let mapi = self.determine_map(&key);
        let mut submap = self.submaps[mapi].write();
        submap.insert(key, value);
    }

    #[inline]
    pub fn contains_key(&self, key: &K) -> bool {
        let mapi = self.determine_map(&key);
        let submap = self.submaps[mapi].read();
        submap.contains_key(&key)
    }

    #[inline]
    pub fn get(&'a self, key: &'a K) -> Option<DHashMapRef<'a, K, V>> {
        let mapi = self.determine_map(&key);
        let submap = self.submaps[mapi].read();
        if submap.contains_key(&key) {
            Some(DHashMapRef { lock: submap, key })
        } else {
            None
        }
    }

    #[inline]
    pub fn get_mut(&'a self, key: &'a K) -> Option<DHashMapRefMut<'a, K, V>> {
        let mapi = self.determine_map(&key);
        let submap = self.submaps[mapi].write();
        if submap.contains_key(&key) {
            Some(DHashMapRefMut { lock: submap, key })
        } else {
            None
        }
    }

    #[inline]
    pub fn remove(&self, key: &K) -> Option<(K, V)> {
        let mapi = self.determine_map(&key);
        let mut submap = self.submaps[mapi].write();
        submap.remove_entry(key)
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

    #[inline(always)]
    pub fn submaps_read(
        &self,
    ) -> impl Iterator<Item = parking_lot::RwLockReadGuard<HashMap<K, V>>> {
        self.submaps.iter().map(RwLock::read)
    }

    #[inline(always)]
    pub fn submaps_write(
        &self,
    ) -> impl Iterator<Item = parking_lot::RwLockWriteGuard<HashMap<K, V>>> {
        self.submaps.iter().map(RwLock::write)
    }

    #[inline(always)]
    pub fn determine_map(&self, key: &K) -> usize {
        let mut hash_state = fxhash::FxHasher64::default();
        hash_state.write_u64(self.hash_nonce);
        key.hash(&mut hash_state);

        let hash = hash_state.finish();
        let shift = 64 - NCB;

        (hash >> shift) as usize
    }
}

#[inline(always)]
fn check_opt(ncb: u64, ncm: usize) -> bool {
    2_u64.pow(ncb as u32) == ncm as u64
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

    #[inline(always)]
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

    #[inline(always)]
    fn deref(&self) -> &V {
        self.lock.get(self.key).unwrap()
    }
}

impl<'a, K, V> DerefMut for DHashMapRefMut<'a, K, V>
where
    K: Hash + Eq,
{
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut V {
        self.lock.get_mut(self.key).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_assert_64() {
        let map = DHashMap::new();

        for i in 0..64_i32 {
            map.insert(i, i * 2);
        }

        for i in 0..64_i32 {
            assert_eq!(i * 2, *map.get(&i).unwrap());
        }
    }
}

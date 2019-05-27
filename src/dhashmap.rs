//! Please see the struct level documentation.

use hashbrown::HashMap;
use crate::parking_lot::RwLock;
use crate::std::hash::Hash;
use crate::std::hash::Hasher;
use crate::std::ops::{Deref, DerefMut};
use crate::std::vec::Vec;
use crate::std::boxed::Box;

/// The amount of bits to look at when determining maps. Default
const NCB: u64 = 8;

/// DHashMap is a threadsafe concurrent hashmap with good allround performance and is tuned for both reads and writes.
///
/// The API mostly matches that of the standard library hashmap but there are some
/// differences to due to the design of the hashmap.
///
/// Due to design limitations you cannot iterate over the map normally. Please use one of the below iterator functions to iterate over contained
/// subtables and then iterate over those.
///
/// Unsafe is used in all operations that require accessing a subtables to avoid bounds checking.
/// This is guaranteed to be safe since we cannot possibly get a value higher than the amount of subtables.
/// The amount of subtables cannot be altered after creation in any way.

pub struct DHashMap<K, V>
where
    K: Hash + Eq,
{
    ncb: usize,
    submaps: Box<[RwLock<HashMap<K, V>>]>,
    hash_nonce: u64,
}

impl<'a, K: 'a, V: 'a> DHashMap<K, V>
where
    K: Hash + Eq,
{
    /// Create a new DHashMap. Doesn't allocate space for elements until it starts filling up.
    /// The amount of submaps used is based on the formula 2^n where n is the value passed. 256 or 8 as a value is the default.
    ///
    /// Will panic if the first parameter plugged into the formula 2^n produces a result higher than isize::MAX.
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", inline(always))]
    pub fn new(submaps_exp_of_two_pow: usize) -> Self {
        let ncm = 1 << submaps_exp_of_two_pow;

        Self {
            ncb: submaps_exp_of_two_pow,
            submaps: (0..ncm).map(|_| RwLock::new(HashMap::new())).collect::<Vec<_>>().into_boxed_slice(),
            hash_nonce: rand::random(),
        }
    }

    /// Create a new DHashMap with a specified capacity.
    ///
    /// Will panic if the first parameter plugged into the formula 2^n produces a result higher than isize::MAX.
    #[cfg(feature = "std")]
    #[cfg_attr(feature = "std", inline(always))]
    pub fn with_capacity(submaps_exp_of_two_pow: usize, capacity: usize) -> Self {

        let ncm = 1 << submaps_exp_of_two_pow;
        let cpm = capacity / ncm;

        Self {
            ncb: submaps_exp_of_two_pow,
            submaps: (0..ncm).map(|_| RwLock::new(HashMap::with_capacity(cpm))).collect::<Vec<_>>().into_boxed_slice(),
            hash_nonce: rand::random(),
        }
    }

    /// Create a new DHashMap with a specified nonce. Not recommended.
    ///
    /// Will panic if the first parameter plugged into the formula 2^n produces a result higher than isize::MAX.
    pub fn with_nonce(submaps_exp_of_two_pow: usize, hash_nonce: u64) -> Self {
        let ncm = 1 << submaps_exp_of_two_pow;

        Self {
            ncb: submaps_exp_of_two_pow,
            submaps: (0..ncm).map(|_| RwLock::new(HashMap::new())).collect::<Vec<_>>().into_boxed_slice(),
            hash_nonce,
        }
    }

    /// Insert an element into the map.
    #[inline(always)]
    pub fn insert(&self, key: K, value: V) {
        let mapi = self.determine_map(&key);
        let mut submap = unsafe { self.submaps.get_unchecked(mapi).write() };
        submap.insert(key, value);
    }

    /// Check if the map contains the specified key.
    #[inline(always)]
    pub fn contains_key(&self, key: &K) -> bool {
        let mapi = self.determine_map(&key);
        let submap = unsafe { self.submaps.get_unchecked(mapi).read() };
        submap.contains_key(&key)
    }

    /// Get a shared reference to an element contained within the map.
    #[inline(always)]
    pub fn get(&'a self, key: &'a K) -> Option<DHashMapRef<'a, K, V>> {
        let mapi = self.determine_map(&key);
        let submap = unsafe { self.submaps.get_unchecked(mapi).read() };
        if submap.contains_key(&key) {
            Some(DHashMapRef { lock: submap, key })
        } else {
            None
        }
    }

    /// Get a unique reference to an element contained within the map.
    #[inline(always)]
    pub fn get_mut(&'a self, key: &'a K) -> Option<DHashMapRefMut<'a, K, V>> {
        let mapi = self.determine_map(&key);
        let submap = unsafe { self.submaps.get_unchecked(mapi).write() };
        if submap.contains_key(&key) {
            Some(DHashMapRefMut { lock: submap, key })
        } else {
            None
        }
    }

    /// Remove an element from the map if it exists. Will return the K, V pair.
    #[inline(always)]
    pub fn remove(&self, key: &K) -> Option<(K, V)> {
        let mapi = self.determine_map(&key);
        let mut submap = unsafe { self.submaps.get_unchecked(mapi).write() };
        submap.remove_entry(key)
    }

    /// Retain all elements that the specified function returns `true` for.
    #[inline(always)]
    pub fn retain<F: Clone + FnMut(&K, &mut V) -> bool>(&self, f: F) {
        self.submaps.iter().for_each(|locked| {
            let mut submap = locked.write();
            submap.retain(f.clone());
        });
    }

    /// Clear all elements from the map.
    #[inline(always)]
    pub fn clear(&self) {
        self.submaps.iter().for_each(|locked| {
            let mut submap = locked.write();
            submap.clear();
        });
    }

    /// Apply a function to every item in the map.
    #[inline(always)]
    pub fn alter<F: FnMut((&K, &mut V)) + Clone>(&self, f: F) {
        self.tables_write().for_each(|mut t| t.iter_mut().for_each(f.clone()))
    }

    /// Iterate over submaps in a read only fashion.
    #[inline(always)]
    pub fn tables_read(
        &self,
    ) -> impl Iterator<Item = SMRInterface<K, V>> {
        self.submaps.iter().map(|t| SMRInterface::new(t.read()))
    }

    /// Iterate over submaps in a read-write fashion.
    #[inline(always)]
    pub fn tables_write(
        &self,
    ) -> impl Iterator<Item = SMRWInterface<K, V>> {
        self.submaps.iter().map(|t| SMRWInterface::new(t.write()))
    }

    #[inline(always)]
    pub fn determine_map(&self, key: &K) -> usize {
        let mut hash_state = fxhash::FxHasher64::default();
        hash_state.write_u64(self.hash_nonce);
        key.hash(&mut hash_state);

        let hash = hash_state.finish();
        let shift = 64 - self.ncb;

        (hash >> shift) as usize
    }
}

impl<K, V> Default for DHashMap<K, V>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self::new(NCB as usize)
    }
}

pub struct SMRInterface<'a, K, V>
where
    K: Hash + Eq,
{
    inner: crate::parking_lot::RwLockReadGuard<'a, HashMap<K, V>>,
}

impl<'a, K: 'a, V: 'a> SMRInterface<'a, K, V>
where
    K: Hash + Eq,
{
    fn new(inner: crate::parking_lot::RwLockReadGuard<'a, HashMap<K, V>>) -> Self {
        Self {
            inner,
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter()
    }
}

pub struct SMRWInterface<'a, K, V>
where
    K: Hash + Eq,
{
    inner: crate::parking_lot::RwLockWriteGuard<'a, HashMap<K, V>>,
}

impl<'a, K: 'a, V: 'a> SMRWInterface<'a, K, V>
where
    K: Hash + Eq,
{
    fn new(inner: crate::parking_lot::RwLockWriteGuard<'a, HashMap<K, V>>) -> Self {
        Self {
            inner,
        }
    }

    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter()
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.inner.iter_mut()
    }
}

pub struct DHashMapRef<'a, K, V>
where
    K: Hash + Eq,
{
    pub lock: crate::parking_lot::RwLockReadGuard<'a, HashMap<K, V>>,
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
    pub lock: crate::parking_lot::RwLockWriteGuard<'a, HashMap<K, V>>,
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
    fn insert_then_assert_1024() {
        let map = DHashMap::new(4);

        for i in 0..1024_i32 {
            map.insert(i, i * 2);
        }

        map.alter(|(_, v)| *v *= 2);

        for i in 0..1024_i32 {
            assert_eq!(i * 4, *map.get(&i).unwrap());
        }
    }
}

use super::*;
use std::hash::Hash;
use hashbrown::HashMap;
use std::cell::{RefCell, Ref, RefMut};
use std::ops::{Deref, DerefMut};
use ccl_owning_ref::OwningRef;

pub enum InterfaceError {
    Other,
    InvalidKey,
}

pub type InterfaceResult<T> = Result<T, InterfaceError>;

pub struct DHashMapInterfaceRef<'a, K: Hash + Eq, V> {
    v: OwningRef<Ref<'a, Lock<'a, K, V>>, V>,
}

impl<'a, K: Hash + Eq, V> DHashMapInterfaceRef<'a, K, V> {
    pub fn value(&self) -> &V {
        &*self.v
    }
}

impl<'a, K: Hash + Eq, V> Deref for DHashMapInterfaceRef<'a, K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.value()
    }
}

enum Lock<'a, K: Hash + Eq, V> {
    Read(parking_lot::RwLockReadGuard<'a, HashMap<K, V>>),
    Write(parking_lot::RwLockWriteGuard<'a, HashMap<K, V>>),
}

impl<'a, K: Hash + Eq, V> Lock<'a, K, V> {
    fn read(&self) -> &HashMap<K, V> {
        match self {
            Lock::Read(l) => &*l,
            Lock::Write(l) => &*l,
        }
    }
}

pub struct Interface<'a, K: Hash + Eq, V> {
    map: &'a DHashMap<K, V>,
    locks: Box<[Option<RefCell<Lock<'a, K, V>>>]>,
}

impl<'a, K: Hash + Eq, V> Interface<'a, K, V> {
    pub(crate) fn new(map: &'a DHashMap<K, V>) -> Self {
        let locks = (0..map.chunks_count()).map(|_| None).collect::<Vec<_>>().into_boxed_slice();

        Self {
            map,
            locks,
        }
    }

    pub fn get(&'a mut self, key: &K) -> InterfaceResult<DHashMapInterfaceRef<'a, K, V>> {
        let idx = self.map.determine_map(key);
        self.fetch_lock(idx, false);
        let map = self.locks[idx].as_ref().unwrap().borrow();

        if map.read().contains_key(key) {
            let or = OwningRef::new(map);
            let or = or.map(|v| v.read().get(key).unwrap());
            Ok(DHashMapInterfaceRef { v: or })
        } else {
            Err(InterfaceError::InvalidKey)
        }
    }

    fn fetch_lock(&mut self, idx: usize, writable: bool) {
        if let None = &mut self.locks[idx] {
            let l = if writable {
                Lock::Write(self.map.get_submap(idx).write())
            } else {
                Lock::Read(self.map.get_submap(idx).read())
            };

            self.locks[idx] = Some(RefCell::new(l));
        }
    }
}

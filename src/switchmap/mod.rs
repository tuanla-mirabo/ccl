mod vecmap;

use crate::util;
use hashbrown::HashMap;
use std::borrow::Borrow;
use std::hash::Hash;
use vecmap::VecMap;

const THRESHOLD: usize = 64;

pub enum SwitchMap<K: Hash + Eq, V> {
    Flat(VecMap<K, V>),
    Map(HashMap<K, V>),
}

impl<K: Hash + Eq, V> SwitchMap<K, V> {
    pub fn new() -> Self {
        SwitchMap::Flat(VecMap::new())
    }

    pub fn with_capacity(c: usize) -> Self {
        if c > THRESHOLD {
            SwitchMap::Map(HashMap::with_capacity(c))
        } else {
            SwitchMap::Flat(VecMap::with_capacity(c))
        }
    }

    pub fn do_check_convert(&mut self) {
        let mut doswitch = false;

        if let SwitchMap::Flat(m) = &self {
            if m.should_convert() {
                doswitch = true;
            }
        }

        if doswitch {
            unsafe {
                util::map_in_place(self, |m| match m {
                    SwitchMap::Flat(m) => SwitchMap::Map(m.into_hashmap()),
                    SwitchMap::Map(m) => SwitchMap::Map(m),
                });
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            SwitchMap::Flat(m) => m.len(),
            SwitchMap::Map(m) => m.len(),
        }
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        match self {
            SwitchMap::Flat(m) => m.insert(k, v).map(|v| v.1),
            SwitchMap::Map(m) => m.insert(k, v),
        }
    }

    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self {
            SwitchMap::Flat(m) => m.get(k),
            SwitchMap::Map(m) => m.get(k),
        }
    }

    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self {
            SwitchMap::Flat(m) => m.get_mut(k),
            SwitchMap::Map(m) => m.get_mut(k),
        }
    }

    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.get(k).is_some()
    }

    pub fn remove<Q>(&mut self, k: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self {
            SwitchMap::Flat(m) => m.remove(k).map(|v| v.1),
            SwitchMap::Map(m) => m.remove(k),
        }
    }
}

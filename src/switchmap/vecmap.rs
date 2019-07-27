use hashbrown::HashMap;
use std::borrow::Borrow;
use std::hash::Hash;
use std::mem;

pub struct VecMap<K: Hash + Eq, V> {
    keys: Vec<K>,
    values: Vec<V>,
}

impl<K: Hash + Eq, V> VecMap<K, V> {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn with_capacity(c: usize) -> Self {
        Self {
            keys: Vec::with_capacity(c),
            values: Vec::with_capacity(c),
        }
    }

    pub fn should_convert(&self) -> bool {
        self.keys.len() <= super::THRESHOLD
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn insert(&mut self, k: K, mut v: V) -> Option<(K, V)> {
        let keyiter = self.keys.iter_mut();
        let valueiter = self.values.iter_mut();
        let mut iter = keyiter.zip(valueiter);

        for (k1, v1) in iter {
            if *k1 == k {
                mem::swap(v1, &mut v);
                return Some((k, v));
            }
        }

        self.keys.push(k);
        self.values.push(v);

        None
    }

    pub fn remove<Q>(&mut self, k: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        let mut r = None;
        for (i, k1) in self.keys.iter().enumerate() {
            if k1.borrow() == k {
                r = Some(i);
                break;
            }
        }
        if let Some(i) = r {
            Some((self.keys.remove(i), self.values.remove(i)))
        } else {
            None
        }
    }

    pub fn get<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        for (i, k1) in self.keys.iter().enumerate() {
            if k1.borrow() == k {
                return self.values.get(i);
            }
        }
        None
    }

    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Eq,
    {
        for (i, k1) in self.keys.iter().enumerate() {
            if k1.borrow() == k {
                return self.values.get_mut(i);
            }
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.keys.iter().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.keys.iter().zip(self.values.iter_mut())
    }

    pub fn into_hashmap(self) -> HashMap<K, V> {
        let mut map = HashMap::with_capacity(self.keys.len());
        self.keys
            .into_iter()
            .zip(self.values.into_iter())
            .for_each(|(k, v)| {
                map.insert(k, v);
            });
        map
    }
}

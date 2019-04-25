use std::hash::Hash;
use std::hash::Hasher;
use smallvec::SmallVec;
use parking_lot::Mutex;
use std::mem;
use std::ops::{Deref, DerefMut};

// optimization ideas, smallvec for storing small amount inline, only use vecs when multiple elements, no bounds checking, switch to rwlocks

pub const TABLE_AMOUNT: usize = 4; // log2 of actual amount
pub const DEFAULT_TABLE_CAPACITY: usize = 2; // log2 of actual amount
pub const LOAD_FACTOR: f64 = 0.85;

fn calculate_index(hash: u64, bits: usize) -> usize {
    (hash % (1 << bits)) as usize
}

fn calculate_hash<K: Hash>(key: &K) -> u64 {
    let mut hash_state = fxhash::FxHasher64::default();
    key.hash(&mut hash_state);
    hash_state.finish()
}

struct Table<K, V>
where
    K: Hash + Eq
{
    data: Vec<Vec<Entry<K, V>>>,
    capacity: usize, // amount of bits looked at, the max amount of elements is equal to 1 << capacity
    amount: usize, // amount of occupied entries
}

impl<K, V> Table<K, V>
where
    K: Hash + Eq
{
    fn new(capacity: usize) -> Self {
        let true_capacity = 1 << capacity;
        let mut storage = Vec::with_capacity(true_capacity);

        for _ in 0..true_capacity {
            storage.push(Vec::new())
        }

        Self {
            data: storage,
            capacity,
            amount: 0,
        }
    }

    fn load_factor(&self) -> f64 {
        let true_capacity = 1 << self.capacity;
        self.amount as f64 / f64::from(true_capacity)
    }

    fn is_overloaded(&self) -> bool {
        self.load_factor() > LOAD_FACTOR
    }

    fn insert(&mut self, k: K, v: V, hash: u64) {
        let index = calculate_index(hash, self.capacity);
        let evec = &mut self.data[index];

        let was_empty = evec.is_empty();

        let entry = Entry {
            key: k,
            value: v,
        };

        evec.push(entry);
        self.amount += 1;

        if !was_empty && self.is_overloaded() {
            self.realloc(self.capacity + 1)
        }
    }

    fn insert_no_check(&mut self, k: K, v: V, hash: u64) {
        let index = calculate_index(hash, self.capacity);
        let evec = &mut self.data[index];
        let entry = Entry {
            key: k,
            value: v,
        };
        evec.push(entry);
        self.amount += 1;
    }

    fn realloc(&mut self, new_capacity: usize) {
        let new_true_capacity = 1 << new_capacity;
        let mut new_storage = Vec::with_capacity(new_true_capacity);
        for _ in 0..new_true_capacity {
            new_storage.push(Vec::new())
        }

        self.capacity = new_capacity;
        let old_storage = mem::replace(&mut self.data, new_storage);
        self.amount = 0;

        for entry in old_storage.into_iter().flatten() {
            let hash = calculate_hash(&entry.key);
            self.insert_no_check(entry.key, entry.value, hash);
        }
    }

    fn len(&self) -> usize {
        self.amount
    }

    fn capacity(&self) -> usize {
        1 << self.capacity
    }

    fn find_location(&self, hash: u64) -> Option<(usize, usize)> {
        let primary_index = calculate_index(hash, self.capacity);
        let evec = &self.data[primary_index];
        for (i, entry) in evec.iter().enumerate() {
            let ekh = calculate_hash(&entry.key);
            if ekh == hash {
                return Some((primary_index, i));
            }
        }
        None
    }

    fn get_with_location(&self, location: (usize, usize)) -> &Entry<K, V> {
        let evec = &self.data[location.0];
        &evec[location.1]
    }
}

#[derive(PartialEq, Eq)]
struct Entry<K, V>
where
    K: Hash + Eq
{
    key: K,
    value: V,
}

#[derive(Debug)]
pub struct TableStat {
    len: usize,
    capacity: usize
}

#[derive(Debug)]
pub struct DHashMap2Stat {
    pub tables: Vec<TableStat>,
}

/// Highly experimental v2 of DHashMap.
#[derive(Default)]
pub struct DHashMap2<K, V>
where
    K: Hash + Eq
{
    tables: SmallVec<[Mutex<Table<K, V>>; TABLE_AMOUNT]>,
}

impl<K, V> DHashMap2<K, V>
where
    K: Hash + Eq
{
    pub fn new() -> Self {
        let true_table_amount = 1 << TABLE_AMOUNT;
        let tables = (0..true_table_amount).map(|_| Mutex::new(Table::new(DEFAULT_TABLE_CAPACITY))).collect();

        Self {
            tables,
        }
    }

    pub fn stat(&self) -> DHashMap2Stat {
        let mut stat = DHashMap2Stat {
            tables: Vec::new(),
        };

        for locked_table in &self.tables {
            let table = locked_table.lock();
            let tablestat = TableStat {
                len: table.len(),
                capacity: table.capacity(),
            };
            stat.tables.push(tablestat);
        }

        stat
    }

    pub fn insert(&self, k: K, v: V) {
        let hash = calculate_hash(&k);
        let index = calculate_index(hash, TABLE_AMOUNT);
        self.tables[index].lock().insert(k, v, hash);
    }

    pub fn get(&self, k: &K) -> Option<DHashMap2Ref<K, V>> {
        let hash = calculate_hash(&k);
        let index = calculate_index(hash, TABLE_AMOUNT);
        let lock = self.tables[index].lock();
        if let Some(location) = lock.find_location(hash) {
            Some(DHashMap2Ref {
                lock,
                location,
            })
        } else {
            None
        }
    }
}

pub struct DHashMap2Ref<'a, K, V>
where
    K: Hash + Eq,
{
    lock: parking_lot::MutexGuard<'a, Table<K, V>>,
    location: (usize, usize),
}

impl<'a, K, V> Deref for DHashMap2Ref<'a, K, V>
where
    K: Hash + Eq,
{
    type Target = V;

    #[inline(always)]
    fn deref(&self) -> &V {
        &self.lock.get_with_location(self.location).value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_stat_64() {
        let map = DHashMap2::new();

        for i in 0..64 {
            map.insert(i, i * 2);
        }

        println!("Map statistics: {:?}", map.stat());
    }
}

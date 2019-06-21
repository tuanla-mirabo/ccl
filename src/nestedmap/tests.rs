use super::*;
use rayon::prelude::*;

#[test]
fn insert_then_assert_st() {
    let map = NestedMap::default();

    for i in 0..1024_i32 {
        map.insert(i, i * 7);
    }

    for i in 0..1024_i32 {
        assert_eq!(i * 7, *map.get(&i).unwrap());
    }
}

#[test]
fn insert_rayon() {
    let map = NestedMap::default();

    let iter_c: i32 = 1024;

    (0..iter_c).into_par_iter().for_each(|i| {
        map.insert(i, i * 7);
    });
}

#[test]
fn len() {
    let map = NestedMap::default();

    for i in 0..1024_i32 {
        map.insert(i, i);
    }

    assert_eq!(map.len(), 1024);
}

#[test]
fn iter_count_fold() {
    let map = NestedMap::default();

    for i in 0..1024_i32 {
        map.insert(i, i);
    }

    for r in map.iter() {
        assert!(*r >= 0 && *r < 1024);
    }

    assert_eq!(map.iter().count(), 1024);
}

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
fn is_empty() {
    let map: NestedMap<i32, i32> = NestedMap::new_layer_prefill();

    assert_eq!(map.is_empty(), true);
}

#[test]
fn iter_count_fold() {
    let map = NestedMap::new_layer_prefill();

    for i in 0..1024_i32 {
        map.insert(i, i);
    }

    for r in map.iter() {
        assert!(*r >= 0 && *r < 1024);
    }

    assert_eq!(map.iter().count(), 1024);
}

#[test]
fn intoiter() {
    let map = NestedMap::new_layer_prefill();

    for i in 0..1024_i32 {
        map.insert(i, i);
    }

    for r in &map {
        assert!(*r >= 0 && *r < 1024);
    }

    assert_eq!(map.iter().count(), 1024);
}

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

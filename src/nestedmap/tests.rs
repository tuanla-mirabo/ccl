use super::*;

#[test]
fn insert_then_assert_1024() {
    let map = NestedMap::default();

    for i in 0..1024_i32 {
        map.insert(i, i * 7);
    }

    for i in 0..1024_i32 {
        assert_eq!(i * 7, *map.get(&i).unwrap());
    }
}

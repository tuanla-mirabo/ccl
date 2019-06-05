#[macro_use]
extern crate criterion;

use rayon::prelude::*;
use ccl::dhashmap::DHashMap;
use ccl::experimental::crude::CrudeHashMap;
use criterion::Criterion;
use lazy_static::lazy_static;

const DATA1: [u128; 16] = [18, 38, 86182734, 9491, 8471, 98591, 9, 871, 98123, 98391, 9863, 1982, 9386923, 1986, 9824, 1982];
const DATA2: u64 = 192;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

lazy_static! {
    static ref DATA1MAP: DHashMap<u64, [u128; 16]> = dhashmap_ccl_rayon_insert_only_100k_u64_u128x16();
    static ref DATA2MAP: DHashMap<u64, u64> = dhashmap_ccl_rayon_insert_only_100k_u64_u64();
    static ref DATA3MAP: CrudeHashMap<u64, u64> = crudemap_ccl_rayon_insert_only_100k_u64_u64();
}

fn crudemap_ccl_rayon_insert_only_100k_u64_u64() -> CrudeHashMap<u64, u64> {
    let map = CrudeHashMap::new(2048);

    (0..100000_u64).into_par_iter().for_each(|i| {
        map.insert(i, DATA2);
    });

    map
}

fn crudemap_ccl_rayon_read_only_100k_u64_u64(map: &CrudeHashMap<u64, u64>) {
    (0..100000_u64).into_par_iter().for_each(|i| {
        assert!(*map.get(&i).unwrap() != std::u64::MAX);
    });
}

fn dhashmap_ccl_rayon_insert_only_100k_u64_u64() -> DHashMap<u64, u64> {
    let map = DHashMap::with_capacity(8, 100000);

    (0..100000_u64).into_par_iter().for_each(|i| {
        map.insert(i, DATA2);
    });

    map
}

fn dhashmap_ccl_rayon_insert_only_100k_u64_u128x16() -> DHashMap<u64, [u128; 16]> {
    let map = DHashMap::with_capacity(8, 100000);

    (0..100000_u64).into_par_iter().for_each(|i| {
        map.insert(i, DATA1);
    });

    map
}

fn dhashmap_ccl_rayon_read_only_100k_u64_u64(map: &DHashMap<u64, u64>) {
    (0..100000_u64).into_par_iter().for_each(|i| {
        assert!(*map.get(&i).unwrap() == DATA2);
    });
}

fn dhashmap_ccl_rayon_read_only_100k_u64_u128x16(map: &DHashMap<u64, [u128; 16]>) {
    (0..100000_u64).into_par_iter().for_each(|i| {
        assert!(*map.get(&i).unwrap() == DATA1);
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("dhashmap_ccl_rayon_insert_only_100k_u64_u64", |b| b.iter(|| dhashmap_ccl_rayon_insert_only_100k_u64_u64()));
    c.bench_function("crudemap_ccl_rayon_insert_only_100k_u64_u64", |b| b.iter(|| crudemap_ccl_rayon_insert_only_100k_u64_u64()));
    c.bench_function("dhashmap_ccl_rayon_insert_only_100k_u64_u128x16", |b| b.iter(|| dhashmap_ccl_rayon_insert_only_100k_u64_u128x16()));
    c.bench_function("dhashmap_ccl_rayon_read_only_100k_u64_u64", |b| b.iter(|| dhashmap_ccl_rayon_read_only_100k_u64_u64(&DATA2MAP)));
    c.bench_function("crudemap_ccl_rayon_read_only_100k_u64_u64", |b| b.iter(|| crudemap_ccl_rayon_read_only_100k_u64_u64(&DATA3MAP)));
    c.bench_function("dhashmap_ccl_rayon_read_only_100k_u64_u128x16", |b| b.iter(|| dhashmap_ccl_rayon_read_only_100k_u64_u128x16(&DATA1MAP)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

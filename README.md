# ccl

blazing fast data structures for Rust

## performance?

Here are some benchmarks for the concurrent `DHashMap` hashmap. You can run them yourself by executing `cargo bench` in the ccl-bench directory.

```
Xeon 2.1Ghz 16C/32T Hetzner Cloud CXX51

dhashmap_ccl_rayon_insert_only_100k_u64_u64                                                                             
                        time:   [1.7019 ms 1.7097 ms 1.7176 ms]

dhashmap_ccl_rayon_insert_only_100k_u64_u128x16                                                                             
                        time:   [2.3916 ms 2.4040 ms 2.4173 ms]

dhashmap_ccl_rayon_read_only_100k_u64_u64                                                                             
                        time:   [1.2478 ms 1.2487 ms 1.2497 ms]

dhashmap_ccl_rayon_read_only_100k_u64_u128x16                                                                             
                        time:   [1.5210 ms 1.5259 ms 1.5329 ms]
```

## todo list

- add a new interface api for managing borrows to prevent deadlocks

- explore new concurrent hashmap designs possibly based on c++ junction

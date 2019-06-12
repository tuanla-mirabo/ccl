# ccl

Fast datastructures for use in highly concurrent systems.

## Performance

Please refer to the Grand Concurrent Hashmap Competition for performance regarding maps. https://gitlab.nebulanet.cc/xacrimon/rs-hm-bench

Benchmarks for other parts of the library are a work in progress.

## Todo list

* [ ] Ergonomic multiborrow API for `DHashMap`

* [ ] NestedMap + NestedSet

* [ ] Concurrent LIFO stack

* [ ] Concurrent unordered element list (Like the use case a `Vec` may provide for single threaded scenarios). Mostly for grouping data.

* [ ] Crossbeam integration

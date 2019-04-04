#![feature(await_macro, async_await, futures_api)]

//! ccl is a library with fast and concurrect data structures for rust
//! at the moment ccl requires the use of a nightly toolchain

pub mod dhashmap;
pub mod timedcache;
pub mod hltimedcache;

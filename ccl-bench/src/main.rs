#![feature(const_fn)]

use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{RwLock, Mutex};
use std::collections::HashMap;
use lazy_static::lazy_static;
use ccl::dhashmap::DHashMap;
use std::alloc::System;

lazy_static! {
    static ref MEM: Mutex<u64> = Mutex::new(0);
}

pub struct Trallocator<A: GlobalAlloc>(pub A, AtomicU64);

unsafe impl<A: GlobalAlloc> GlobalAlloc for Trallocator<A> {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        self.1.fetch_add(l.size() as u64, Ordering::SeqCst);
        self.0.alloc(l)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, l: Layout) {
        self.0.dealloc(ptr, l);
        self.1.fetch_sub(l.size() as u64, Ordering::SeqCst);
    }
}

impl<A: GlobalAlloc> Trallocator<A> {
    pub const fn new(a: A) -> Self {
        Trallocator(a, AtomicU64::new(0))
    }

    pub fn reset(&self) {
        self.1.store(0, Ordering::SeqCst);
    }
    pub fn get(&self) -> u64 {
        self.1.load(Ordering::SeqCst)
    }
}

#[global_allocator]
static GLOBAL: Trallocator<System> = Trallocator::new(System);

fn rwlock_hashmap() {
    let map = RwLock::new(HashMap::new());
    for i in 0..75000 {
        map.write().unwrap().insert(i, i * 8);
    }
    *MEM.lock().unwrap() = GLOBAL.get();
}

fn dhashmap() {
    let map = DHashMap::default();
    for i in 0..75000 {
        map.insert(i, i * 8);
    }
    *MEM.lock().unwrap() = GLOBAL.get();
}

fn main() {
    rwlock_hashmap();
    println!("mem heap usage rwlock_hashmap (KiB): {}", *MEM.lock().unwrap() / 1024);
    dhashmap();
    println!("mem heap usage dhashmap (KiB): {}", *MEM.lock().unwrap() / 1024);
}

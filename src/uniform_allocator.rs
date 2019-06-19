use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

// TODO: write v3
//
//
// v3 ideas:
//
// 1. linked list of segments, each containing a bitmap to keep track of whats allocated and an array of buckets, segments dont have to be a linked list
// some kind of tree might work aswell
//
// 2. a stack containing pointers to free chunks within a segment
// to allocate just pop the stack, to dealloc, push to the stack, have multiple segments
//
//
// Unrelated idea: cache parts in a threadlocal cache, this maybe be segments or whatever

pub struct UniformAllocator<T> {
    ctr: Box<[AtomicUsize]>,
    marker: PhantomData<T>,
}

impl<T> UniformAllocator<T> {
    #[inline]
    pub fn new(count: usize) -> Self {
        Self {
            ctr: (0..count)
                .map(|_| AtomicUsize::new(0))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            marker: PhantomData,
        }
    }

    #[inline]
    pub fn alloc(&self, tag: usize) -> *mut u8 {
        self.ctr[tag % self.ctr.len()].fetch_add(1, Ordering::Relaxed);
        unsafe { alloc(Layout::new::<T>()) }
    }

    #[inline]
    pub fn dealloc(&self, tag: usize, ptr: *mut u8) -> Option<T> {
        self.ctr[tag % self.ctr.len()].fetch_add(1, Ordering::Relaxed);
        unsafe {
            let data = ptr::read(ptr as *const _);
            dealloc(ptr, Layout::new::<T>());
            data
        }
    }
}

impl<T> Default for UniformAllocator<T> {
    fn default() -> Self {
        Self::new(num_cpus::get() * 4)
    }
}

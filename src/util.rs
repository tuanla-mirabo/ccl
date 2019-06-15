use crate::uniform_allocator::UniformAllocator;
use crossbeam_epoch::{self as epoch, Atomic, Owned, Pointer, Shared};
use std::hash::{Hash, Hasher};
use std::ptr;
use std::sync::atomic::Ordering;

pub trait UniformAllocExt<T> {
    fn uniform_alloc(allocator: &UniformAllocator<T>, tag: usize, v: T) -> Self;
}

pub trait UniformDeallocExt<T> {
    fn uniform_dealloc(&self, allocator: &UniformAllocator<T>, tag: usize) -> Option<T>;
}

impl<T> UniformAllocExt<T> for Atomic<T> {
    #[inline]
    fn uniform_alloc(allocator: &UniformAllocator<T>, tag: usize, v: T) -> Self {
        let ptr = allocator.alloc(tag) as usize;
        unsafe {
            ptr::write(ptr as *mut T, v);
            let atomicptr = Atomic::null();
            atomicptr.store(Shared::from_usize(ptr), Ordering::SeqCst);
            atomicptr
        }
    }
}

impl<T> UniformDeallocExt<T> for Atomic<T> {
    #[inline]
    fn uniform_dealloc(&self, allocator: &UniformAllocator<T>, tag: usize) -> Option<T> {
        unsafe {
            let ptr = self
                .load(Ordering::SeqCst, epoch::unprotected())
                .into_usize() as *mut u8;
            allocator.dealloc(tag, ptr)
        }
    }
}

impl<T> UniformAllocExt<T> for Owned<T> {
    #[inline]
    fn uniform_alloc(allocator: &UniformAllocator<T>, tag: usize, v: T) -> Self {
        let ptr = allocator.alloc(tag) as usize;
        unsafe {
            ptr::write(ptr as *mut T, v);
            Owned::from_usize(ptr)
        }
    }
}

impl<'a, T> UniformDeallocExt<T> for Shared<'a, T> {
    #[inline]
    fn uniform_dealloc(&self, allocator: &UniformAllocator<T>, tag: usize) -> Option<T> {
        let ptr = self.clone().into_usize();
        allocator.dealloc(tag, ptr as *mut u8)
    }
}

#[inline]
pub fn hash_with_nonce<T: Hash>(v: &T, nonce: u8) -> u64 {
    let mut hasher = seahash::SeaHasher::new();
    hasher.write_u8(nonce);
    v.hash(&mut hasher);
    hasher.finish()
}

#[inline]
pub fn sharedptr_null<'a, T>() -> Shared<'a, T> {
    unsafe { Shared::from_usize(0) }
}

use std::mem;
use slab::Slab;
use hashbrown::HashMap;
use std::collections::VecDeque;
use parking_lot::Mutex;

#[derive(Hash, PartialEq, Eq)]
struct ObjectKey(usize);

#[derive(Hash, PartialEq, Eq)]
struct Pointer(usize);

struct SlabSegment<T> {
    objects: Slab<T>,
    mappings: HashMap<Pointer, ObjectKey>,
}

impl<T> SlabSegment<T> {
    fn new(capacity: usize) -> Self {
        Self {
            objects: Slab::with_capacity(capacity),
            mappings: HashMap::with_capacity(capacity),
        }
    }

    fn has_space(&self) -> bool {
        self.objects.len() < self.objects.capacity()
    }

    fn alloc(&mut self) -> *mut u8 {
        let key = self.objects.insert(unsafe { mem::zeroed() });
        let ptr = unsafe { self.objects.get_unchecked_mut(key) as *mut T as usize };
        self.mappings.insert(Pointer(ptr), ObjectKey(key));
        ptr as *mut u8
    }

    fn dealloc(&mut self, ptr: *mut u8) -> bool {
        let ptr = ptr as usize;
        if let Some(key) = self.mappings.remove(&Pointer(ptr)) {
            self.objects.remove(key.0);
            true
        } else {
            false
        }
    }
}

struct MemoryPool<T> {
    segments: VecDeque<SlabSegment<T>>,
}

impl<T> MemoryPool<T> {
    fn new() -> Self {
        Self {
            segments: VecDeque::new(),
        }
    }

    fn alloc(&mut self) -> *mut u8 {
        let mut search_idx = 0;

        loop {
            if let Some(mut segment) = self.segments.get_mut(search_idx) {
                if segment.has_space() {
                    return segment.alloc();
                } else {
                    search_idx += 1;
                }
            } else {
                self.segments.push_back(SlabSegment::new(64));
            }
        }
    }

    fn dealloc(&mut self, ptr: *mut u8) {
        for segment in &mut self.segments {
            if segment.dealloc(ptr) {
                break;
            }
        }
    }
}

pub struct UniformAllocator<T> {
    pool_count: usize,
    pools: Box<[Mutex<MemoryPool<T>>]>,
}

impl<T> UniformAllocator<T> {
    pub fn new(pool_count: usize) -> Self {
        Self {
            pool_count,
            pools: (0..pool_count).map(|_| Mutex::new(MemoryPool::new())).collect::<Vec<_>>().into_boxed_slice(),
        }
    }

    pub fn alloc(&self, tag: usize) -> *mut u8 {
        let mut pool = self.pools[tag % self.pool_count].lock();
        pool.alloc()
    }

    pub fn dealloc(self, tag: usize, ptr: *mut u8) {
        let mut pool = self.pools[tag % self.pool_count].lock();
        pool.dealloc(ptr);
    }
}

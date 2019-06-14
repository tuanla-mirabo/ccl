use std::mem;
use slab::Slab;
use hashbrown::HashMap;

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

    fn dealloc(&mut self, ptr: *mut u8) {
        let ptr = ptr as usize;
        let key = self.mappings.remove(&Pointer(ptr)).unwrap();
        self.objects.remove(key.0);
    }
}

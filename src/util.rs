use std::hash::{Hash, Hasher};
use crossbeam_epoch::{Shared, Pointer};

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

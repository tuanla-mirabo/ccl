use std::hash::Hash;
use std::hash::Hasher;
use crossbeam_epoch::{Shared, Pointer};

#[inline]
pub fn hash_with_nonce<T: Hash>(v: &T, nonce: u8) -> u64 {
    let mut hash_state = fxhash::FxHasher64::default();
    hash_state.write_u8(nonce);
    v.hash(&mut hash_state);
    let result = hash_state.finish();

    let mut hash_state = fxhash::FxHasher64::default();
    hash_state.write_u64(result.wrapping_mul(nonce.into()));
    hash_state.write_u8(nonce);
    hash_state.finish()
}

#[inline]
pub fn sharedptr_null<'a, T>() -> Shared<'a, T> {
    unsafe { Shared::from_usize(0) }
}

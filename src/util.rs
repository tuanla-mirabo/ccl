use std::hash::Hash;
use std::hash::Hasher;
use std::sync::atomic::{AtomicBool, Ordering};

#[inline]
pub fn hash<T: Hash>(v: &T) -> u64 {
    let mut hash_state = fxhash::FxHasher64::default();
    v.hash(&mut hash_state);
    hash_state.finish()
}

#[inline]
pub fn hash_with_nonce<T: Hash>(v: &T, nonce: u64) -> u64 {
    let mut hash_state = fxhash::FxHasher64::default();
    hash_state.write_u64(nonce);
    v.hash(&mut hash_state);
    hash_state.finish()
}

#[inline]
pub fn round_pow2(x: usize) -> usize {
    let mut pow = 1;
    while pow < x {
        pow *= 2;
    }
    pow
}

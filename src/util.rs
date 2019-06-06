use std::hash::Hash;
use std::hash::Hasher;
use std::sync::atomic::{Ordering, AtomicBool};

#[inline]
pub fn hash<T: Hash>(v: &T) -> u64 {
    let mut hash_state = fxhash::FxHasher64::default();
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

#[inline]
pub fn atomic_spin_while_true(x: &AtomicBool) {
    while x.load(Ordering::SeqCst) {}
}

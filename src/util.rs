use std::hash::Hash;
use std::hash::Hasher;

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

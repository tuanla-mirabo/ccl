use std::hash::Hash;
use std::hash::Hasher;

#[inline]
pub fn hash_with_nonce<T: Hash>(v: &T, nonce: u64) -> u64 {
    let mut hash_state = fxhash::FxHasher64::default();
    hash_state.write_u64(nonce);
    v.hash(&mut hash_state);
    let result = hash_state.finish();

    let mut hash_state = fxhash::FxHasher64::default();
    hash_state.write_u64(result.wrapping_mul(nonce));
    hash_state.write_u64(nonce);
    hash_state.write_u64(nonce.wrapping_add(result));
    hash_state.finish()
}

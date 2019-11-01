use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

// QuickCheck doesn't generate uniform integer input, so let's hash
// the blocks before turning them into a bitset.
pub fn hash_u64(x: u64) -> u64 {
    let mut h = DefaultHasher::new();
    h.write_u64(x);
    h.finish()
}

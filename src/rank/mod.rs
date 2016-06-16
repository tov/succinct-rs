//! Support for fast rank queries.

use storage::BitStore;

mod jacobson;
pub use self::jacobson::*;

/// Interface for types that support rank queries.
pub trait RankSupport : BitStore {
    /// Returns the rank at a given position.
    ///
    /// This is the number of 1s up to and including that position.
    fn rank(&self, position: u64) -> u64;

    /// Returns the rank of 0s at a given position.
    ///
    /// This is the number of 0s up to and including that position.
    fn rank0(&self, position: u64) -> u64 {
        position + 1 - self.rank(position)
    }
}

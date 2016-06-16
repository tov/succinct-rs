//! Support for fast select queries.

use storage::BitStore;

mod bin_search;
pub use self::bin_search::*;

/// Interface for types that support select queries.
pub trait SelectSupport : BitStore {
    /// Returns the position of the `index`th 1 bit.
    fn select(&self, index: u64) -> Option<u64>;
}


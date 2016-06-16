//! Data structure to support fast select queries.

/// Interface for types that support select queries.
pub trait Select {
    /// Returns the position of the `index`th 1 bit.
    fn select(&self, index: u64) -> u64;
}


/// Supports selecting for 1 bits.
pub trait Select1Support {
    /// Returns the position of the `index`th 1 bit.
    fn select1(&self, index: u64) -> Option<u64>;
}

/// Supports selecting for 0 bits.
pub trait Select0Support {
    /// Returns the position of the `index`th 0 bit.
    fn select0(&self, index: u64) -> Option<u64>;
}

/// Supports select queries over associated type `Over`.
pub trait SelectSupport {
    /// The type of value that we can search for.
    type Over: Copy;

    /// Returns the position of the `index`th occurrence of `value`.
    fn select(&self, index: u64, value: Self::Over) -> Option<u64>;
}

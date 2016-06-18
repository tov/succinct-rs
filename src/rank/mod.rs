//! Support for fast rank queries.

mod jacobson;
pub use self::jacobson::*;

/// Interface for types that support rank queries.
///
/// Associated type `Over` gives the type that we can query about. For
/// example, `RankSupport<Over=bool>` lets us rank `0` and `1`, whereas
/// `RankSupport<Over=u8>` will rank arbitrary bytes.
pub trait RankSupport {
    /// The type of value to rank.
    type Over: Copy;

    /// Returns the rank of the given value at a given position.
    ///
    /// This is the number of occurrences of `value` up to and including
    /// that position.
    fn rank(&self, position: u64, value: Self::Over) -> u64;

    /// The size of the vector being ranked.
    ///
    /// Rank queries beyond this point will continue to return the same
    /// value, so you can think of `limit` as one past the end of the
    /// last place that the rank changes.
    fn limit(&self) -> u64;
}

/// Convenience trait for `RankSupport` over `bool`.
pub trait BitRankSupport : RankSupport<Over = bool> {
    /// Returns the rank of 1 at the given position.
    ///
    /// This is the number of occurrences of 0 up to that position.
    fn rank1(&self, position: u64) -> u64 {
        self.rank(position, true)
    }

    /// Returns the rank of 0 at the given position.
    ///
    /// This is the number of occurrences of 0 up to and including that
    /// position.
    fn rank0(&self, position: u64) -> u64 {
        position + 1 - self.rank1(position)
    }
}

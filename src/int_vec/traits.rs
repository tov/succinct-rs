use storage::BlockType;

/// Types that can be accessed as immutable arrays of integers of
/// limited width.
pub trait IntVec {
    /// The type of primitive value to represent elements.
    type Block: BlockType;

    /// The number of elements.
    fn len(&self) -> u64;

    /// Is the vector empty?
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bit width of each element.
    fn element_bits(&self) -> usize;

    /// Fetches the value of the `index`th element.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    fn get(&self, index: u64) -> Self::Block;
}

/// Types that can be accessed as mutable arrays of integers of limited
/// width.
pub trait IntVecMut: IntVec {
    /// Updates the value of the `index`th element.
    ///
    /// # Panics
    ///
    ///   - Panics if `index` is out of bounds.
    ///
    ///   - May panic (?) if `element_value` is too large to
    ///     fit in the element size. (TODO: What’s the right thing here?)
    fn set(&mut self, index: u64, value: Self::Block);
}

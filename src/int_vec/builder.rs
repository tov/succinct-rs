use std::marker::PhantomData;

use super::*;
use block_type::BlockType;

/// Builder for configuring and constructing an `IntVec`.
#[derive(Clone, Debug)]
pub struct IntVecBuilder<Block: BlockType = usize> {
    /// The number of bits in each element.
    element_bits: usize,
    /// The initial number of elements.
    n_elements: u64,
    /// The number of elements to allocate storage for.
    capacity: u64,
    /// How to initialize the elements.
    fill: Fill<Block>,
    marker: PhantomData<Block>,
}

impl<Block: BlockType> Default for IntVecBuilder<Block> {
    fn default() -> Self {
        IntVecBuilder::<Block>::new(Block::nbits())
    }
}

impl<Block: BlockType> IntVecBuilder<Block> {
    /// Creates a new `IntVecBuilder` with `element_bits` bits per
    /// elements.
    pub fn new(element_bits: usize) -> Self {
        assert!(element_bits != 0,
                "IntVecBuilder: cannot have 0-bit elements.");
        IntVecBuilder {
            element_bits: element_bits,
            n_elements: 0,
            capacity: 0,
            fill: Fill::Block(Block::zero()),
            marker: PhantomData,
        }
    }

    /// Builds the specified `IntVec`.
    ///
    /// # Panics
    ///
    /// Panics if the size conditions of [`IntVec::<Block>::is_okay_size()`](struct.IntVec.html#method.is_okay_size) are not met.
    pub fn build(&self) -> IntVec<Block> {
        let n_blocks
            = IntVec::<Block>::compute_n_blocks(self.element_bits,
                                                self.capacity).unwrap();

        let mut result = IntVec {
            blocks: Vec::with_capacity(n_blocks),
            n_elements: 0,
            element_bits: self.element_bits,
        };

        result.resize(self.n_elements, self.fill);

        result
    }

    /// Sets the element size to `element_bits`.
    ///
    /// The elements will range from `0` to `2.pow(element_bits) - 1`.
    pub fn element_bits(&mut self, element_bits: usize) -> &mut Self {
        self.element_bits = element_bits;
        self
    }

    /// Sets the initial number of elements.
    ///
    /// If `n_elements()` finds that `capacity()` has been set to a
    /// lower value, it adjust `capacity()` upward.
    pub fn n_elements(&mut self, n_elements: u64) -> &mut Self {
        self.n_elements = n_elements;
        if self.n_elements > self.capacity {
            self.capacity = self.n_elements;
        }
        self
    }

    /// Sets the size of the initial allocation, which may be larger
    /// than the initial number of elements.
    ///
    /// If `capacity()` finds that `n_elements()` has been set to a
    /// higher value, it adjust `n_elements()` downward.
    pub fn capacity(&mut self, capacity: u64) -> &mut Self {
        self.capacity = capacity;
        if self.capacity < self.n_elements {
            self.n_elements = capacity;
        }
        self
    }

    /// Zero-fill the new vector’s data.
    pub fn zero_fill(&mut self) -> &mut Self {
        self.fill = Fill::Block(Block::zero());
        self
    }

    /// Fill the vector’s data with the specified block. This will align
    /// as a block, which may not align with elements in any particular way.
    /// It’s not yet specified how the elements are laid out.
    pub fn block_fill(&mut self, block: Block) -> &mut Self {
        self.fill = Fill::Block(block);
        self
    }

    /// Fill the vector’s data with the given element.
    pub fn element_fill(&mut self, element: Block) -> &mut Self {
        self.fill = Fill::Element(element);
        self
    }
}

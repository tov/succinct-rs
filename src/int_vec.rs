//! Bit-packed vectors of `N`-bit unsigned integers.

use std::marker::PhantomData;
use std::mem;

use num::{PrimInt, CheckedMul, ToPrimitive};

use typenum::{NonZero, Unsigned};
pub use typenum::{U1, U2, U3, U4, U5, U6, U7, U8, U9, U10, U11, U12,
                  U13, U14, U15, U16, U17, U18, U19, U20, U21, U22, U23,
                  U24, U25, U26, U27, U28, U29, U30, U31, U32, U33, U34,
                  U35, U36, U37, U38, U39, U40, U41, U42, U43, U44, U45,
                  U46, U47, U48, U49, U50, U51, U52, U53, U54, U55, U56,
                  U57, U58, U59, U60, U61, U62, U63, U64, };

pub trait BlockType: PrimInt {
    #[inline]
    fn element_mask(element_bits: usize) -> Self {
        (Self::one() << element_bits) - Self::one()
    }

    #[inline]
    fn get_bits(self, start: usize, len: usize) -> Self {
        let block_bits = 8 * mem::size_of::<Self>();
        let limit      = start + len;
        (self >> (block_bits - limit)) & Self::element_mask(len)
    }
}

impl<Block: PrimInt> BlockType for Block { }

/// A vector of `N`-bit unsigned integers.
///
/// `Block` gives the representation type. `N` must not exceed the number
/// of bits in `Block`.
#[derive(Clone)]
pub struct IntVec<N: NonZero + Unsigned, Block: BlockType = usize> {
    blocks: Box<[Block]>,
    n_elements: usize,
    marker: PhantomData<N>,
}

#[derive(Clone, Copy, Debug)]
struct Address {
    block_index: usize,
    bit_offset: usize,
}

impl<N, Block> IntVec<N, Block>
        where N: NonZero + Unsigned, Block: PrimInt {

    #[inline]
    fn block_bytes() -> usize {
        mem::size_of::<Block>()
    }

    /// The number of bits per block of storage.
    #[inline]
    pub fn block_bits() -> usize {
        8 * Self::block_bytes()
    }

    /// The number of bits per elements.
    #[inline]
    pub fn element_bits() -> usize {
        N::to_usize()
    }

    /// True if elements are packed one per block.
    #[inline]
    pub fn is_packed() -> bool {
        Self::element_bits() == Self::block_bits()
    }

    /// True if elements are aligned within blocks.
    #[inline]
    pub fn is_aligned() -> bool {
        Self::block_bits() % Self::element_bits() == 0
    }

    #[inline]
    fn element_address(&self, element_index: usize) -> Address {
        // Because of the underlying slice, this bounds checks twice.
        assert!(element_index < self.n_elements,
                "IntVec: index out of bounds.");

        if Self::is_packed() {
            Address {
                block_index: element_index,
                bit_offset: 0,
            }
        } else {
            let element_index = element_index as u64;
            let element_bits  = Self::element_bits() as u64;
            let block_bits    = Self::block_bits() as u64;

            let bit_index     = element_index * element_bits;

            Address {
                block_index: (bit_index / block_bits) as usize,
                bit_offset: (bit_index % block_bits) as usize,
            }
        }
    }

    // Computes the block size while carefully avoiding overflow.
    // Provided we can do this without overflowing at construction time,
    // we shouldn’t have to check for overflow for indexing after that.
    #[inline]
    fn compute_block_size(n_elements: usize) -> Option<usize> {
        let n_elements   = n_elements as u64;
        let element_bits = Self::element_bits() as u64;
        let block_bits   = Self::block_bits() as u64;

        assert!(block_bits >= element_bits,
                "Element bits cannot exceed block bits");

        if let Some(n_bits) = n_elements.checked_mul(element_bits) {
            let mut result = n_bits / block_bits;
            if n_bits % block_bits == 0 { result += 1 }
            result.to_usize()
        } else { None }
    }

    /// Creates a new vector to hold the given number of elements.
    ///
    /// # Panics
    ///
    /// Panics if any of:
    ///
    ///   - `block_bits() < N`;
    ///   - `n_elements * N` doesn’t fit in a `u64`; or
    ///   - `ceiling(n_elements * N / block_bits())` doesn’t fit in a `usize`.
    pub fn new(n_elements: usize) -> Self {
        let block_size = Self::compute_block_size(n_elements)
            .expect("IntVec: size overflow");

        let mut vec = Vec::with_capacity(n_elements);
        vec.resize(block_size, Block::zero());

        IntVec {
            blocks: vec.into_boxed_slice(),
            n_elements: n_elements,
            marker: PhantomData,
        }
    }

    /// Returns the number of elements in the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.n_elements
    }

    /// Returns the element at the given index.
    pub fn get(&self, element_index: usize) -> Block {
        if Self::is_packed() {
            return self.blocks[element_index];
        }

        let block_bits = Self::block_bits();
        let element_bits = Self::element_bits();

        let address = self.element_address(element_index);
        let margin = block_bits - address.bit_offset;

        if margin <= element_bits {
            let block = self.blocks[address.block_index];
            return block.get_bits(address.bit_offset, element_bits)
        }

        let extra = element_bits - margin;

        let block1 = self.blocks[address.block_index];
        let block2 = self.blocks[address.block_index + 1];

        let high_bits = block1.get_bits(address.bit_offset, margin);
        let low_bits = block2.get_bits(0, extra);

        (high_bits << extra) | low_bits
    }

}


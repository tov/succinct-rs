//! Bit-packed vectors of `N`-bit unsigned integers.

use std::marker::PhantomData;
use std::mem;

use num::{PrimInt, ToPrimitive};

use typenum::{NonZero, Unsigned};
pub use typenum::{U1, U2, U3, U4, U5, U6, U7, U8, U9, U10, U11, U12,
                  U13, U14, U15, U16, U17, U18, U19, U20, U21, U22, U23,
                  U24, U25, U26, U27, U28, U29, U30, U31, U32, U33, U34,
                  U35, U36, U37, U38, U39, U40, U41, U42, U43, U44, U45,
                  U46, U47, U48, U49, U50, U51, U52, U53, U54, U55, U56,
                  U57, U58, U59, U60, U61, U62, U63, U64, };

mod block_type;
pub use self::block_type::*;

/// A vector of `N`-bit unsigned integers.
///
/// `Block` gives the representation type. `N` must not exceed the number
/// of bits in `Block`.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct IntVec<N: NonZero + Unsigned, Block: BlockType = usize> {
    blocks: Box<[Block]>,
    n_elements: usize,
    marker: PhantomData<N>,
}

/// A `IntVec` of `1`-bit integers is a bit vector.
pub type BitVec<Block = usize> = IntVec<U1, Block>;

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

    #[inline]
    fn bit_address(&self, bit_index: usize) -> Address {
        // TODO: bounds check (since the slice might have extra space)

        Address {
            block_index: bit_index / Self::block_bits(),
            bit_offset: bit_index % Self::block_bits(),
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

        debug_assert!(block_bits >= element_bits,
                      "Element bits cannot exceed block bits");

        if let Some(n_bits) = n_elements.checked_mul(element_bits) {
            let mut result = n_bits / block_bits;
            if n_bits % block_bits > 0 { result += 1 }
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

    /// Is the vector empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the element at the given index.
    pub fn get(&self, element_index: usize) -> Block {
        if Self::is_packed() {
            return self.blocks[element_index];
        }

        let element_bits = Self::element_bits();

        if element_bits == 1 {
            if self.get_bit(element_index) {
                return Block::one();
            } else {
                return Block::zero();
            }
        }

        let block_bits = Self::block_bits();

        let address = self.element_address(element_index);
        let margin = block_bits - address.bit_offset;

        if margin >= element_bits {
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

    /// Sets the element at the given index.
    pub fn set(&mut self, element_index: usize, element_value: Block) {
        if Self::is_packed() {
            self.blocks[element_index] = element_value;
            return;
        }

        debug_assert!(element_value < Block::one() << Self::element_bits(),
                      "IntVec::set: value overflow");

        let element_bits = Self::element_bits();

        if element_bits == 1 {
            self.set_bit(element_index, element_value == Block::one());
            return;
        }

        let block_bits = Self::block_bits();

        let address = self.element_address(element_index);
        let margin = block_bits - address.bit_offset;

        if margin >= element_bits {
            let old_block = self.blocks[address.block_index];
            let new_block = old_block.set_bits(address.bit_offset,
                                               element_bits,
                                               element_value);
            self.blocks[address.block_index] = new_block;
            return;
        }

        let extra = element_bits - margin;

        let old_block1 = self.blocks[address.block_index];
        let old_block2 = self.blocks[address.block_index + 1];

        let high_bits = element_value >> extra;

        let new_block1 = old_block1.set_bits(address.bit_offset,
                                             margin, high_bits);
        let new_block2 = old_block2.set_bits(0, extra, element_value);

        self.blocks[address.block_index] = new_block1;
        self.blocks[address.block_index + 1] = new_block2;
    }

    /// Gets the bit at the given position.
    pub fn get_bit(&self, bit_index: usize) -> bool {
        let address = self.bit_address(bit_index);
        let block = self.blocks[address.block_index];
        block.get_bit(address.bit_offset)
    }

    /// Sets the bit at the given position.
    pub fn set_bit(&mut self, bit_index: usize, bit_value: bool) {
        let address = self.bit_address(bit_index);
        let old_block = self.blocks[address.block_index];
        let new_block = old_block.set_bit(address.bit_offset, bit_value);
        self.blocks[address.block_index] = new_block;
    }

    /// Gets an iterator over the elements of the vector.
    pub fn iter(&self) -> Iter<N, Block> {
        Iter {
            vec: self,
            start: 0,
            limit: self.len()
        }
    }
}

/// An iterator over the elements of an [`IntVec`](struct.IntVec.html).
pub struct Iter<'a, N: 'a, Block: 'a = usize>
    where N: NonZero + Unsigned, Block: BlockType
{
    vec: &'a IntVec<N, Block>,
    start: usize,
    limit: usize,
}

impl<'a, N, Block> Iterator for Iter<'a, N, Block>
    where N: NonZero + Unsigned, Block: BlockType
{
    type Item = Block;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.limit {
            let result = self.vec.get(self.start);
            self.start += 1;
            Some(result)
        } else { None }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    fn count(self) -> usize {
        self.len()
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.start = self.start.checked_add(n).unwrap_or(self.limit);
        self.next()
    }
}

impl<'a, N, Block> ExactSizeIterator for Iter<'a, N, Block>
    where N: NonZero + Unsigned, Block: BlockType
{
    fn len(&self) -> usize {
        self.limit - self.start
    }
}

impl<'a, N, Block> DoubleEndedIterator for Iter<'a, N, Block>
    where N: NonZero + Unsigned, Block: BlockType
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start < self.limit {
            self.limit -= 1;
            Some(self.vec.get(self.limit))
        } else { None }

    }
}

impl<'a, N: 'a, Block: 'a> IntoIterator for &'a IntVec<N, Block>
    where N: NonZero + Unsigned, Block: BlockType
{
    type Item = Block;
    type IntoIter = Iter<'a, N, Block>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_empty() {
        let v = IntVec::<U4>::new(0);
        assert!(v.is_empty());
    }

    #[test]
    fn packed() {
        let mut v = IntVec::<U32, u32>::new(10);
        assert_eq!(10, v.len());

        assert_eq!(0, v.get(0));
        assert_eq!(0, v.get(9));

        v.set(0, 89);
        assert_eq!(89, v.get(0));
        assert_eq!(0, v.get(1));

        v.set(0, 56);
        v.set(1, 34);
        assert_eq!(56, v.get(0));
        assert_eq!(34, v.get(1));
        assert_eq!(0, v.get(2));

        v.set(9, 12);
        assert_eq!(12, v.get(9));
    }

    #[test]
    #[should_panic]
    fn packed_oob() {
        let v = IntVec::<U32, u32>::new(10);
        assert_eq!(0, v.get(10));
    }

    #[test]
    fn aligned() {
        let mut v = IntVec::<U4>::new(20);
        assert_eq!(20, v.len());

        assert_eq!(0, v.get(0));
        assert_eq!(0, v.get(9));

        v.set(0, 13);
        assert_eq!(13, v.get(0));
        assert_eq!(0, v.get(1));

        v.set(1, 15);
        assert_eq!(13, v.get(0));
        assert_eq!(15, v.get(1));
        assert_eq!(0, v.get(2));

        v.set(1, 4);
        v.set(19, 9);
        assert_eq!(13, v.get(0));
        assert_eq!(4, v.get(1));
        assert_eq!(0, v.get(2));
        assert_eq!(9, v.get(19));
    }

    #[test]
    #[should_panic]
    fn aligned_oob() {
        let v = IntVec::<U4>::new(20);
        assert_eq!(0, v.get(20));
    }

    #[test]
    fn unaligned() {
        let mut v = IntVec::<U5>::new(20);
        assert_eq!(20, v.len());

        assert_eq!(0, v.get(0));
        assert_eq!(0, v.get(9));

        v.set(0, 13);
        assert_eq!(13, v.get(0));
        assert_eq!(0, v.get(1));

        v.set(1, 15);
        assert_eq!(13, v.get(0));
        assert_eq!(15, v.get(1));
        assert_eq!(0, v.get(2));

        v.set(1, 4);
        v.set(19, 9);
        assert_eq!(13, v.get(0));
        assert_eq!(4, v.get(1));
        assert_eq!(0, v.get(2));
        assert_eq!(9, v.get(19));
    }

    #[test]
    #[should_panic]
    fn unaligned_oob() {
        let v = IntVec::<U5>::new(20);
        assert_eq!(0, v.get(20));
    }

    #[test]
    fn iter() {
        let mut v = IntVec::<U13, u16>::new(5);
        v.set(0, 1);
        v.set(1, 1);
        v.set(2, 2);
        v.set(3, 3);
        v.set(4, 5);

        assert_eq!(vec![1, 1, 2, 3, 5], v.iter().collect::<Vec<_>>());
    }
}

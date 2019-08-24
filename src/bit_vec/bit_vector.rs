use std::fmt;

#[cfg(target_pointer_width = "32")]
use num::ToPrimitive;

use super::traits::*;
use internal::vector_base::{self, VectorBase};
use space_usage::SpaceUsage;
use storage::BlockType;

/// Uncompressed vector of bits.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BitVector<Block: BlockType = usize>(VectorBase<Block>);

impl<Block: BlockType> BitVector<Block> {
    /// Creates a new, empty bit vector.
    pub fn new() -> Self {
        BitVector(VectorBase::new())
    }

    /// Creates a new, empty bit vector with space allocated for `capacity`
    /// bits.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is too large. In particular the number of
    /// blocks required by the capacity (`capacity / Block::nbits()`)
    /// must fit in a `usize`.
    pub fn with_capacity(capacity: u64) -> Self {
        BitVector(VectorBase::with_capacity(1, capacity))
    }

    /// Creates a new, empty bit vector with space allocated for `capacity`
    /// blocks.
    pub fn block_with_capacity(capacity: usize) -> Self {
        BitVector(VectorBase::block_with_capacity(capacity))
    }

    /// Creates a new bit vector of `len` bits initialized to `value`.
    ///
    /// # Panics
    ///
    /// Panics if `len` is too large. In particular the number of
    /// blocks required by the capacity (`len / Block::nbits()`)
    /// must fit in a `usize`.
    pub fn with_fill(len: u64, value: bool) -> Self {
        let block_size =
            Block::checked_ceil_div_nbits(len).expect("BitVector::with_fill: overflow");
        let block_value = if value { !Block::zero() } else { Block::zero() };
        let mut result = Self::block_with_fill(block_size, block_value);
        result.0.truncate(1, len);
        result
    }

    /// Creates a new bit vector of `block_len` blocks initialized to `value`.
    pub fn block_with_fill(block_len: usize, value: Block) -> Self {
        BitVector(VectorBase::block_with_fill(1, block_len, value))
    }

    /// How many bits the bit vector can hold without reallocating.
    pub fn capacity(&self) -> u64 {
        self.0.capacity(1)
    }

    /// How many blocks the bit vector can hold without reallocating.
    pub fn block_capacity(&self) -> usize {
        self.0.block_capacity()
    }

    /// Resizes the bit vector to the given number of elements,
    /// filling if necessary.
    ///
    /// # Panics
    ///
    /// Panics if `new_len` is too large. In particular the number of
    /// blocks required by the capacity (`new_len / Block::nbits()`)
    /// must fit in a `usize`.
    pub fn resize(&mut self, new_len: u64, value: bool) {
        let new_block_len =
            Block::checked_ceil_div_nbits(new_len).expect("BitVector::resize: overflow");

        if new_len < self.bit_len() || !value {
            self.block_resize(new_block_len, Block::zero());
        } else {
            let trailing = Block::last_block_bits(self.bit_len());
            let remaining = Block::nbits() - trailing;
            for _ in 0..remaining {
                self.0.push_bit(true);
            }
            self.block_resize(new_block_len, !Block::zero());
        }

        self.0.truncate(1, new_len);
    }

    /// Resizes the bit vector to the given number of blocks,
    /// filling if necessary.
    pub fn block_resize(&mut self, new_len: usize, value: Block) {
        self.0.block_resize(1, new_len, value);
    }

    /// Reserves capacity for at least `additional` more bits to be
    /// inserted.
    ///
    /// The collection may reserve more space to avoid frequent
    /// reallocations.
    ///
    /// # Panics
    ///
    /// Panics if the number of blocks overflows a `usize`.
    pub fn reserve(&mut self, additional: u64) {
        self.0.reserve(1, additional);
    }

    /// Reserves capacity for at least `additional` blocks of bits to be
    /// inserted.
    ///
    /// The collection may reserve more space to avoid frequent
    /// reallocations.
    ///
    /// # Panics
    ///
    /// Panics if the number of blocks overflows a `usize`.
    pub fn block_reserve(&mut self, additional: usize) {
        self.0.block_reserve(additional);
    }

    /// Reserves capacity for at least `additional` more bits to be
    /// inserted.
    ///
    /// Unlike [`reserve`](#method.reserve), does nothing if the
    /// capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the number of blocks overflows a `usize`.
    pub fn reserve_exact(&mut self, additional: u64) {
        self.0.reserve_exact(1, additional);
    }

    /// Reserves capacity for at least `additional` more blocks of bits to be
    /// inserted.
    ///
    /// Unlike [`reserve_block`](#method.reserve_block), does nothing if the
    /// capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the number of blocks overflows a `usize`.
    pub fn block_reserve_exact(&mut self, additional: usize) {
        self.0.block_reserve_exact(additional);
    }

    /// Shrinks the capacity to just fit the number of elements.
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit()
    }

    /// Shrinks to the given size.
    ///
    /// Does nothing if `len` is greater than the current size.
    pub fn truncate(&mut self, len: u64) {
        self.0.truncate(1, len);
    }

    /// Shrinks to the given size in blocks.
    ///
    /// Does nothing if `block_len` is greater than the current size in blocks.
    pub fn block_truncate(&mut self, block_len: usize) {
        self.0.block_truncate(1, block_len);
    }

    /// Sets the size to 0 while retaining the allocated storage.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Returns an iterator over the bits of the bit vector
    pub fn iter(&self) -> Iter<Block> {
        Iter(vector_base::Iter::new(1, &self.0))
    }
}

impl<Block: BlockType> BitVec for BitVector<Block> {
    type Block = Block;

    #[inline]
    fn bit_len(&self) -> u64 {
        self.0.len()
    }

    fn get_bit(&self, index: u64) -> bool {
        self.0.get_bit(index)
    }

    #[inline]
    fn get_block(&self, index: usize) -> Block {
        self.0.get_block(index)
    }
}

impl<Block: BlockType> BitVecMut for BitVector<Block> {
    fn set_bit(&mut self, index: u64, value: bool) {
        self.0.set_bit(index, value);
    }

    #[inline]
    fn set_block(&mut self, index: usize, value: Block) {
        self.0.set_block(1, index, value);
    }
}

impl<Block: BlockType> BitVecPush for BitVector<Block> {
    fn push_bit(&mut self, value: bool) {
        self.0.push_bit(value);
    }

    fn pop_bit(&mut self) -> Option<bool> {
        self.0.pop_bit()
    }

    fn push_block(&mut self, value: Block) {
        self.0.push_block(1, value);
    }
}

impl<Block: BlockType> fmt::Binary for BitVector<Block> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        for bit in self {
            let bit = if bit { "1" } else { "0" };
            try!(formatter.write_str(bit));
        }

        Ok(())
    }
}

impl<Block: BlockType> SpaceUsage for BitVector<Block> {
    fn is_stack_only() -> bool {
        false
    }

    fn heap_bytes(&self) -> usize {
        self.0.heap_bytes()
    }
}

impl<Block: BlockType> Default for BitVector<Block> {
    fn default() -> Self {
        BitVector::new()
    }
}

/// Iterator over `BitVector`.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Iter<'a, Block: BlockType + 'a = usize>(vector_base::Iter<'a, Block>);

impl<'a, Block: BlockType> Iterator for Iter<'a, Block> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|bit| bit != Block::zero())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize {
        self.0.count()
    }

    fn last(self) -> Option<Self::Item> {
        self.0.last().map(|bit| bit != Block::zero())
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n).map(|bit| bit != Block::zero())
    }
}

#[cfg(target_pointer_width = "64")]
impl<'a, Block: BlockType> ExactSizeIterator for Iter<'a, Block> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a, Block: BlockType> DoubleEndedIterator for Iter<'a, Block> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|bit| bit != Block::zero())
    }
}

impl<'a, Block: BlockType + 'a> IntoIterator for &'a BitVector<Block> {
    type Item = bool;
    type IntoIter = Iter<'a, Block>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod test {
    use bit_vec::*;

    macro_rules! assert_bv {
        ($expected:expr, $actual:expr) => {
            assert_eq!($expected, format!("{:b}", $actual))
        };
    }

    #[test]
    fn new() {
        let bit_vector: BitVector = BitVector::new();
        assert_eq!(0, bit_vector.bit_len());
        assert_eq!(0, bit_vector.block_len());
    }

    #[test]
    fn capacity() {
        let bit_vector: BitVector<u32> = BitVector::new();
        assert_eq!(0, bit_vector.capacity());

        let bit_vector: BitVector<u32> = BitVector::with_capacity(65);
        assert_eq!(96, bit_vector.capacity());
    }

    #[test]
    fn push_binary() {
        let mut bit_vector: BitVector = BitVector::new();
        bit_vector.push_bit(true);
        bit_vector.push_bit(false);
        bit_vector.push_bit(false);
        assert_eq!("100", format!("{:b}", bit_vector));
    }

    #[test]
    fn block_with_fill() {
        let bit_vector: BitVector<u8> = BitVector::block_with_fill(3, 0b101);
        assert_eq!(3, bit_vector.block_capacity());
        assert_bv!("101000001010000010100000", bit_vector);
    }

    #[test]
    fn with_fill() {
        let bv0: BitVector = BitVector::with_fill(20, false);
        let bv1: BitVector = BitVector::with_fill(20, true);

        assert_eq!(false, bv0.get_bit(3));
        assert_eq!(true, bv1.get_bit(3));

        assert_bv!("00000000000000000000", bv0);
        assert_bv!("11111111111111111111", bv1);
    }

    #[test]
    fn push_pop() {
        let mut bit_vector: BitVector = BitVector::new();
        bit_vector.push_bit(true);
        bit_vector.push_bit(false);
        bit_vector.push_bit(false);
        assert_eq!(Some(false), bit_vector.pop_bit());
        assert_eq!(Some(false), bit_vector.pop_bit());
        assert_eq!(Some(true), bit_vector.pop_bit());
        assert_eq!(None, bit_vector.pop_bit());
    }

    #[test]
    fn push_get() {
        let mut bit_vector: BitVector = BitVector::new();
        bit_vector.push_bit(true);
        bit_vector.push_bit(false);
        bit_vector.push_bit(false);
        assert_eq!(3, bit_vector.bit_len());
        assert_eq!(1, bit_vector.block_len());
        assert_eq!(true, bit_vector.get_bit(0));
        assert_eq!(false, bit_vector.get_bit(1));
        assert_eq!(false, bit_vector.get_bit(2));
    }

    #[test]
    #[should_panic]
    fn get_oob() {
        let mut bit_vector: BitVector = BitVector::new();
        bit_vector.push_bit(true);
        bit_vector.get_bit(3);
    }

    #[test]
    fn push_block() {
        let mut bit_vector: BitVector<u32> = BitVector::new();
        bit_vector.push_block(0);
        assert_bv!("00000000000000000000000000000000", bit_vector);
    }

    #[test]
    fn push_bits_get_block() {
        let mut bit_vector: BitVector = BitVector::new();
        bit_vector.push_bit(true); // 1
        bit_vector.push_bit(true); // 2
        bit_vector.push_bit(false); // (4)
        bit_vector.push_bit(false); // (8)
        bit_vector.push_bit(true); // 16

        assert_eq!(19, bit_vector.get_block(0));
    }

    #[test]
    fn push_block_get_block() {
        let mut bit_vector: BitVector = BitVector::new();
        bit_vector.push_block(358);
        bit_vector.push_block(!0);
        assert_eq!(358, bit_vector.get_block(0));
        assert_eq!(!0, bit_vector.get_block(1));
    }

    #[test]
    #[should_panic]
    fn get_block_oob() {
        let mut bit_vector: BitVector = BitVector::new();
        bit_vector.push_bit(true);
        bit_vector.get_block(3);
    }

    #[test]
    fn push_block_get_bit() {
        let mut bit_vector: BitVector = BitVector::new();
        bit_vector.push_block(0b10101);
        assert_eq!(true, bit_vector.get_bit(0));
        assert_eq!(false, bit_vector.get_bit(1));
        assert_eq!(true, bit_vector.get_bit(2));
        assert_eq!(false, bit_vector.get_bit(3));
        assert_eq!(true, bit_vector.get_bit(4));
        assert_eq!(false, bit_vector.get_bit(5));
    }

    #[test]
    fn push_block_set_get() {
        let mut bit_vector: BitVector = BitVector::new();
        bit_vector.push_block(0);
        bit_vector.set_bit(0, true);
        bit_vector.set_bit(1, true);
        bit_vector.set_bit(2, false);
        bit_vector.set_bit(3, true);
        bit_vector.set_bit(4, false);
        assert_eq!(true, bit_vector.get_bit(0));
        assert_eq!(true, bit_vector.get_bit(1));
        assert_eq!(false, bit_vector.get_bit(2));
        assert_eq!(true, bit_vector.get_bit(3));
        assert_eq!(false, bit_vector.get_bit(4));
    }

    #[test]
    fn set_block_mask() {
        let mut bit_vector: BitVector = BitVector::new();

        bit_vector.push_bit(false);
        bit_vector.set_block(0, 0b11);
        assert_eq!(0b01, bit_vector.get_block(0));

        bit_vector.push_bit(false);
        bit_vector.set_block(0, 0b11);
        assert_eq!(0b11, bit_vector.get_block(0));
    }

    #[test]
    fn resize() {
        let mut bit_vector: BitVector<u8> = BitVector::new();

        bit_vector.push_bit(true);
        bit_vector.push_bit(false);
        bit_vector.push_bit(true);
        assert_bv!("101", bit_vector);

        bit_vector.resize(21, false);
        assert_bv!("101000000000000000000", bit_vector);

        bit_vector.resize(22, false);
        assert_bv!("1010000000000000000000", bit_vector);

        bit_vector.resize(5, false);
        assert_bv!("10100", bit_vector);

        bit_vector.resize(21, true);
        assert_bv!("101001111111111111111", bit_vector);

        bit_vector.resize(4, true);
        assert_bv!("1010", bit_vector);

        bit_vector.push_block(0b11111111);
        assert_bv!("1010000011111111", bit_vector);
    }

    #[test]
    fn block_resize() {
        let mut bit_vector: BitVector<u8> = BitVector::new();

        bit_vector.push_bit(true);
        bit_vector.push_bit(false);
        bit_vector.push_bit(true);
        assert_bv!("101", bit_vector);

        bit_vector.block_resize(1, 0);
        assert_bv!("10100000", bit_vector);

        bit_vector.block_resize(3, 0b01000101);
        assert_bv!("101000001010001010100010", bit_vector);

        bit_vector.block_resize(2, 0);
        assert_bv!("1010000010100010", bit_vector);
    }
}

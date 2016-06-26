#![allow(dead_code)]

use bit_vector::{Bits, BitsMut};
use storage::BlockType;

/// VectorBase provides basic functionality for IntVec and BitVec. It
/// doesn’t know its element size, but it does know (once provided its
/// element size) how to maintain the invariants:
///
///  1. All blocks are in use storing elements.
///  2. Any bits not in use are zero.
///
/// These two properties are what make it safe to use derived
/// implementations of Eq, Ord, Hash, etc.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VectorBase<Block> {
    len: u64,
    vec: Vec<Block>,
}

impl<Block: BlockType> VectorBase<Block> {
    // Maintains the second invariant: extra bits are zero.
    #[inline]
    fn clear_extra_bits(&mut self, element_bits: usize) {
        let bit_len = self.len * element_bits as u64;
        self.vec.last_mut().map(|block| {
            let mask = Block::low_mask(Block::last_block_bits(bit_len));
            *block = *block & mask;
        });
    }

    // Sets the length based on the number of blocks in the underlying Vec.
    #[inline]
    fn set_len_from_blocks(&mut self, element_bits: usize) {
        self.len = Block::mul_nbits(self.vec.len()) / element_bits as u64;
    }

    #[inline]
    pub fn new() -> Self {
        Self::with_block_capacity(0)
    }

    #[inline]
    pub fn with_block_capacity(block_capacity: usize) -> Self {
        VectorBase {
            len: 0,
            vec: Vec::with_capacity(block_capacity)
        }
    }

    pub fn with_capacity(element_bits: usize, capacity: u64) -> Self {
        Self::with_block_capacity(
            Block::ceil_div_nbits_checked(element_bits as u64 * capacity)
                .expect("VectorBase::with_capacity: overflow"))
    }

    pub fn with_block_fill(element_bits: usize, block_len: usize, fill: Block)
                           -> Self {
        let mut result = VectorBase {
            len: 0,
            vec: vec![ fill; block_len ],
        };

        result.set_len_from_blocks(element_bits);
        result.clear_extra_bits(element_bits);
        result
    }

    pub fn get_block(&self, block_index: usize) -> Block {
        self.vec[block_index]
    }

    pub fn set_block(&mut self, element_bits: usize,
                     block_index: usize, value: Block) {
        self.vec[block_index] = value;
        if block_index + 1 == self.vec.len() {
            self.clear_extra_bits(element_bits);
        }
    }

    pub fn get_bits(&self, element_bits: usize, index: u64, count: usize)
                    -> Block {
        assert!(index + count as u64 <= self.len * element_bits as u64,
                "VectorBase::get_bits: out of bounds");
        self.vec.get_bits(index, count)
    }

    pub fn set_bits(&mut self, element_bits: usize, index: u64,
                    count: usize, value: Block) {
        assert!(index + count as u64 <= self.len * element_bits as u64,
                "VectorBase::set_bits: out of bounds");
        self.vec.set_bits(index, count, value);
    }

    // PRECONDITION: element_size == 1
    pub fn get_bit(&self, index: u64) -> bool {
        assert!(index < self.len, "VectorBase::get_bit: out of bounds");
        self.vec.get_bit(index)
    }

    // PRECONDITION: element_size == 1
    pub fn set_bit(&mut self, index: u64, value: bool) {
        assert!(index < self.len, "VectorBase::set_bit: out of bounds");
        self.vec.set_bit(index, value);
    }

    pub fn push_block(&mut self, element_bits: usize, value: Block) {
        self.vec.push(value);
        self.set_len_from_blocks(element_bits);
        self.clear_extra_bits(element_bits);
    }

    pub fn pop_block(&mut self, element_bits: usize) -> Option<Block> {
        let result = self.vec.pop();
        self.set_len_from_blocks(element_bits);
        self.clear_extra_bits(element_bits);
        result
    }

    pub fn push_bits(&mut self, element_bits: usize, value: Block) {
        if element_bits as u64 * (self.len + 1) > Block::mul_nbits(self.vec.len()) {
            self.vec.push(Block::zero());
        }

        let pos = self.len;
        self.len = pos + 1;
        self.set_bits(element_bits, pos as u64 * element_bits as u64,
                      element_bits, value);
    }

    pub fn pop_bits(&mut self, element_bits: usize) -> Option<Block> {
        if self.len == 0 { return None; }

        let new_bit_len = element_bits as u64 * (self.len - 1);

        let result = self.get_bits(element_bits, new_bit_len, element_bits);
        self.set_bits(element_bits, new_bit_len, element_bits, Block::zero());
        self.len -= 1;

        let block_len = Block::ceil_div_nbits(new_bit_len);
        if self.vec.len() > block_len { self.vec.pop(); }

        Some(result)
    }

    // PRECONDITION: element_size == 1
    pub fn push_bit(&mut self, value: bool) {
        if self.len + 1 > Block::mul_nbits(self.vec.len()) {
            self.vec.push(Block::zero());
        }

        let pos = self.len;
        self.len = pos + 1;
        self.set_bit(pos, value);
    }

    pub fn pop_bit(&mut self) -> Option<bool> {
        if self.len == 0 { return None; }

        let new_len = self.len - 1;
        let result = self.get_bit(new_len);
        self.set_bit(new_len, false);
        self.len = new_len;

        let block_len = Block::ceil_div_nbits(new_len);
        if self.vec.len() > block_len { self.vec.pop(); }

        Some(result)
    }

    #[inline]
    pub fn block_len(&self) -> usize {
        self.vec.len()
    }

    #[inline]
    pub fn len(&self) -> u64 {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    #[inline]
    pub fn block_capacity(&self) -> usize {
        self.vec.capacity()
    }

    pub fn capacity(&self, element_bits: usize) -> u64 {
        Block::mul_nbits(self.block_capacity()) / element_bits as u64
    }

    pub fn truncate_block(&mut self, element_bits: usize, block_len: usize) {
        if block_len < self.vec.len() {
            self.vec.truncate(block_len);
            self.set_len_from_blocks(element_bits);
            self.clear_extra_bits(element_bits);
        }
    }

    pub fn truncate(&mut self, element_bits: usize, len: u64) {
        if len < self.len {
            let block_len = Block::ceil_div_nbits(len * element_bits as u64);
            self.vec.truncate(block_len);
            self.len = len;
            self.clear_extra_bits(element_bits);
        }
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
    }

    #[inline]
    pub fn reserve_blocks(&mut self, additional: usize) {
        self.vec.reserve(additional);
    }

    #[inline]
    pub fn reserve_exact_blocks(&mut self, additional: usize) {
        self.vec.reserve_exact(additional);
    }

    fn additional_blocks(&self, element_bits: usize, additional: u64)
                         -> usize {
        let goal_elements = self.len + additional;
        let goal_bits = goal_elements * element_bits as u64;
        let goal_blocks = Block::ceil_div_nbits_checked(goal_bits)
                            .expect("VectorBase::reserve_(exact): overflow");
        goal_blocks.saturating_sub(self.vec.capacity())
    }

    pub fn reserve(&mut self, element_bits: usize, additional: u64) {
        let difference = self.additional_blocks(element_bits, additional);
        self.reserve_blocks(difference);
    }

    pub fn reserve_exact(&mut self, element_bits: usize, additional: u64) {
        let difference = self.additional_blocks(element_bits, additional);
        self.reserve_exact_blocks(difference);
    }

    pub fn resize_blocks(&mut self, element_bits: usize,
                         block_len: usize, fill: Block) {
        self.vec.resize(block_len, fill);
        self.set_len_from_blocks(element_bits);
        self.clear_extra_bits(element_bits);
    }

    pub fn resize(&mut self, element_bits: usize, len: u64, fill: Block) {
        let bit_len = element_bits as u64 * len;
        let block_len = Block::ceil_div_nbits_checked(bit_len)
            .expect("VectorBase::resize: overflow");

        let old_len = self.len;
        self.vec.resize(block_len, Block::zero());
        self.len = len;

        if len <= old_len {
            self.clear_extra_bits(element_bits);
        } else {
            for i in self.len .. len {
                self.set_bits(element_bits, i * element_bits as u64,
                              element_bits, fill);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    type VB = VectorBase<u8>;

    #[test]
    fn new() {
        let v = VB::new();
        assert_eq!(0, v.len());
        assert_eq!(0, v.block_len());
        assert_eq!(0, v.capacity(5));
        assert_eq!(0, v.block_capacity());
    }

    #[test]
    fn with_block_capacity() {
        let v = VB::with_block_capacity(7);
        assert_eq!(0, v.len());
        assert_eq!(0, v.block_len());
        assert_eq!(7, v.capacity(8));
        assert_eq!(14, v.capacity(4));
        assert_eq!(11, v.capacity(5));
        assert_eq!(7, v.block_capacity());
    }

    #[test]
    fn with_capacity() {
        let v = VB::with_capacity(5, 5);
        assert_eq!(0, v.len());
        assert_eq!(0, v.block_len());
        assert_eq!(6, v.capacity(5));
        assert_eq!(32, v.capacity(1));
        assert_eq!(4, v.block_capacity());
    }

    #[test]
    fn with_block_fill() {
        let v = VB::with_block_fill(5, 3, 0b01010101);
        assert_eq!(3, v.block_len());
        assert_eq!(4, v.len());
        assert_eq!(3, v.block_capacity());
        assert_eq!(4, v.capacity(5));

        assert_eq!(true, v.get_bit(0));
        assert_eq!(false, v.get_bit(1));
        assert_eq!(true, v.get_bit(2));
        assert_eq!(false, v.get_bit(3));

        assert_eq!(0b01010101, v.get_block(0));
        assert_eq!(0b01010101, v.get_block(1));
        assert_eq!(0b00000101, v.get_block(2));

        assert_eq!(0b10101, v.get_bits(5, 0, 5));
        assert_eq!(0b01010, v.get_bits(5, 1, 5));
        assert_eq!(0b10101, v.get_bits(5, 2, 5));
        assert_eq!(0b01010, v.get_bits(5, 3, 5));
        assert_eq!(0b10101, v.get_bits(5, 4, 5));
        assert_eq!(0b01010, v.get_bits(5, 5, 5));
    }

    #[test]
    fn set_block() {
        let mut v = VB::with_block_fill(5, 3, 0b01010101);
        assert_eq!(0b01010101, v.get_block(0));
        assert_eq!(0b01010101, v.get_block(1));
        assert_eq!(0b00000101, v.get_block(2));

        v.set_block(5, 2, 0b11111111);
        assert_eq!(0b00001111, v.get_block(2));
    }
}

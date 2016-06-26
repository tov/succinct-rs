#![allow(dead_code)]

#[cfg(target_pointer_width = "32")]
use num::ToPrimitive;

use bit_vector::{Bits, BitsMut};
use space_usage::SpaceUsage;
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
///
/// Many `VectorBase` methods take `element_bits` as a parameter. For methods
/// that create a vector, `element_bits` is checked for overflow. For other methods,
/// it is assumed to have already been checked, so the client must ensure that it
/// doesn’t pass bogus `element_bits` values.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VectorBase<Block> {
    len: u64,
    vec: Vec<Block>,
}

#[inline]
fn len_to_block_len<Block: BlockType>(element_bits: usize, len: u64) -> Option<usize> {
    len.checked_mul(element_bits as u64)
       .and_then(Block::checked_ceil_div_nbits)
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
        self.clear_extra_bits(element_bits);
    }

    #[inline]
    pub fn new() -> Self {
        Self::block_with_capacity(0)
    }

    #[inline]
    pub fn block_with_capacity(block_capacity: usize) -> Self {
        VectorBase {
            len: 0,
            vec: Vec::with_capacity(block_capacity)
        }
    }

    #[inline]
    pub fn with_capacity(element_bits: usize, capacity: u64) -> Self {
        Self::block_with_capacity(
            len_to_block_len::<Block>(element_bits, capacity)
                .expect("VectorBase::with_capacity: overflow"))
    }

    #[inline]
    pub fn block_with_fill(element_bits: usize, block_len: usize, fill: Block)
                           -> Self {
        let mut result = VectorBase {
            len: 0,
            vec: vec![ fill; block_len ],
        };

        result.set_len_from_blocks(element_bits);
        result
    }

    #[inline]
    pub fn with_fill(element_bits: usize, len: u64, value: Block) -> Self {
        let block_len = len_to_block_len::<Block>(element_bits, len)
                            .expect("VectorBase::with_fill: overflow");
        let mut result = VectorBase {
            len: len,
            vec: vec![ Block::zero(); block_len ],
        };

        for i in 0 .. len {
            result.set_bits(element_bits, i * element_bits as u64,
                            element_bits, value);
        }

        result
    }

    #[inline]
    pub fn get_block(&self, block_index: usize) -> Block {
        self.vec[block_index]
    }

    #[inline]
    pub fn set_block(&mut self, element_bits: usize,
                     block_index: usize, value: Block) {
        self.vec[block_index] = value;
        if block_index + 1 == self.vec.len() {
            self.clear_extra_bits(element_bits);
        }
    }

    #[inline]
    pub fn get_bits(&self, element_bits: usize, index: u64, count: usize)
                    -> Block {
        // If element_bits is legit then the RHS of the comparison can't overflow.
        assert!(index + count as u64 <= self.len * element_bits as u64,
                "VectorBase::get_bits: out of bounds");
        self.vec.get_bits(index, count)
    }

    #[inline]
    pub fn set_bits(&mut self, element_bits: usize, index: u64,
                    count: usize, value: Block) {
        // If element_bits is legit then the RHS of the comparison can't overflow.
        assert!(index + count as u64 <= self.len * element_bits as u64,
                "VectorBase::set_bits: out of bounds");
        self.vec.set_bits(index, count, value);
    }

    // PRECONDITION: element_bits == 1
    #[inline]
    pub fn get_bit(&self, index: u64) -> bool {
        assert!(index < self.len, "VectorBase::get_bit: out of bounds");
        self.vec.get_bit(index)
    }

    // PRECONDITION: element_bits == 1
    #[inline]
    pub fn set_bit(&mut self, index: u64, value: bool) {
        assert!(index < self.len, "VectorBase::set_bit: out of bounds");
        self.vec.set_bit(index, value);
    }

    #[inline]
    pub fn push_block(&mut self, element_bits: usize, value: Block) {
        self.vec.push(value);
        self.set_len_from_blocks(element_bits);
    }

    #[inline]
    pub fn pop_block(&mut self, element_bits: usize) -> Option<Block> {
        let result = self.vec.pop();
        self.set_len_from_blocks(element_bits);
        result
    }

    #[inline]
    pub fn push_bits(&mut self, element_bits: usize, value: Block) {
        if element_bits as u64 * (self.len + 1) > Block::mul_nbits(self.vec.len()) {
            self.vec.push(Block::zero());
        }

        let pos = self.len;
        self.len = pos + 1;
        self.set_bits(element_bits, pos as u64 * element_bits as u64,
                      element_bits, value);
    }

    #[inline]
    pub fn pop_bits(&mut self, element_bits: usize) -> Option<Block> {
        if self.len == 0 { return None; }

        let bit_len = element_bits as u64 * (self.len - 1);
        let block_len = Block::ceil_div_nbits(bit_len);

        let result = self.get_bits(element_bits, bit_len, element_bits);
        self.set_bits(element_bits, bit_len, element_bits, Block::zero());
        self.len -= 1;

        if self.vec.len() > block_len { self.vec.pop(); }

        Some(result)
    }

    // PRECONDITION: element_bits == 1
    #[inline]
    pub fn push_bit(&mut self, value: bool) {
        if self.len + 1 > Block::mul_nbits(self.vec.len()) {
            self.vec.push(Block::zero());
        }

        let pos = self.len;
        self.len = pos + 1;
        self.set_bit(pos, value);
    }

    #[inline]
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

    #[inline]
    pub fn capacity(&self, element_bits: usize) -> u64 {
        Block::mul_nbits(self.block_capacity()) / element_bits as u64
    }

    #[inline]
    pub fn block_truncate(&mut self, element_bits: usize, block_len: usize) {
        if block_len < self.vec.len() {
            self.vec.truncate(block_len);
            self.set_len_from_blocks(element_bits);
        }
    }

    #[inline]
    pub fn truncate(&mut self, element_bits: usize, len: u64) {
        if len < self.len {
            let block_len = Block::ceil_div_nbits(len * element_bits as u64);
            self.vec.truncate(block_len);
            self.len = len;
            self.clear_extra_bits(element_bits);
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.vec.clear();
        self.len = 0;
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
    }

    #[inline]
    pub fn block_reserve(&mut self, additional: usize) {
        self.vec.reserve(additional);
    }

    #[inline]
    pub fn block_reserve_exact(&mut self, additional: usize) {
        self.vec.reserve_exact(additional);
    }

    fn additional_blocks(&self, element_bits: usize, additional: u64)
                         -> usize {
        self.len.checked_add(additional)
                .and_then(|e| e.checked_mul(element_bits as u64))
                .and_then(Block::checked_ceil_div_nbits)
                .expect("VectorBase::reserve_(exact): overflow")
                .saturating_sub(self.vec.capacity())
    }

    #[inline]
    pub fn reserve(&mut self, element_bits: usize, additional: u64) {
        let difference = self.additional_blocks(element_bits, additional);
        self.block_reserve(difference);
    }

    #[inline]
    pub fn reserve_exact(&mut self, element_bits: usize, additional: u64) {
        let difference = self.additional_blocks(element_bits, additional);
        self.block_reserve_exact(difference);
    }

    #[inline]
    pub fn block_resize(&mut self, element_bits: usize,
                         block_len: usize, fill: Block) {
        self.vec.resize(block_len, fill);
        self.set_len_from_blocks(element_bits);
    }

    #[inline]
    pub fn resize(&mut self, element_bits: usize, len: u64, fill: Block) {
        let block_len = len_to_block_len::<Block>(element_bits, len)
                            .expect("VectorBase::resize: overflow");

        self.vec.resize(block_len, Block::zero());
        let old_len = self.len;
        self.len = len;

        if len <= old_len {
            self.clear_extra_bits(element_bits);
        } else {
            for i in old_len .. len {
                self.set_bits(element_bits, i * element_bits as u64,
                              element_bits, fill);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Iter<'a, Block: BlockType + 'a> {
    start: u64,
    limit: u64,
    element_bits: usize,
    data:  &'a VectorBase<Block>,
}

impl<'a, Block: BlockType> Iter<'a, Block> {
    #[inline]
    pub fn new(element_bits: usize, data: &'a VectorBase<Block>) -> Self {
        Iter {
            start: 0,
            limit: data.len(),
            element_bits: element_bits,
            data: data,
        }
    }
}

impl<'a, Block: BlockType> Iterator for Iter<'a, Block> {
    type Item = Block;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.limit {
            let result = self.data.get_bits(
                self.element_bits,
                self.element_bits as u64 * self.start,
                self.element_bits);
            self.start += 1;
            Some(result)
        } else { None }
    }

    #[cfg(target_pointer_width = "32")]
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(len) = (self.limit - self.start).to_usize() {
            (len, Some(len))
        } else {
            (0, None)
        }
    }

    #[cfg(target_pointer_width = "64")]
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.start = self.start.checked_add(n as u64).unwrap_or(self.limit);
        self.next()
    }
}

#[cfg(target_pointer_width = "64")]
impl<'a, Block: BlockType> ExactSizeIterator for Iter<'a, Block> {
    #[inline]
    fn len(&self) -> usize {
        (self.limit - self.start) as usize
    }
}

impl<'a, Block: BlockType> DoubleEndedIterator for Iter<'a, Block> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start < self.limit {
            self.limit -= 1;
            Some(self.data.get_bits(
                self.element_bits,
                self.element_bits as u64 * self.limit,
                self.element_bits))
        } else { None }
    }
}

impl<Block: BlockType> SpaceUsage for VectorBase<Block> {
    #[inline]
    fn is_stack_only() -> bool { false }

    #[inline]
    fn heap_bytes(&self) -> usize {
        self.vec.heap_bytes()
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
    fn block_with_capacity() {
        let v = VB::block_with_capacity(7);
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
    fn block_with_fill() {
        let v = VB::block_with_fill(5, 3, 0b01010101);
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
    fn block_with_fill_7() {
        let v = VB::block_with_fill(7, 3, 0b01010101);
        assert_eq!(0b01010101, v.get_block(0));
        assert_eq!(0b01010101, v.get_block(1));
        assert_eq!(0b00010101, v.get_block(2));
    }

    #[test]
    fn with_fill() {
        let mut v = VB::with_fill(5, 5, 0b10110);
        assert_eq!(5, v.len());
        assert_eq!(4, v.block_len());
        for _ in 0 .. 5 {
            assert_eq!(Some(0b10110), v.pop_bits(5));
        }
        assert_eq!(0, v.len());
    }

    #[test]
    fn set_block_5() {
        let mut v = VB::block_with_fill(5, 3, 0b01010101);
        assert_eq!(0b01010101, v.get_block(0));
        assert_eq!(0b01010101, v.get_block(1));
        assert_eq!(0b00000101, v.get_block(2));

        v.set_block(5, 2, 0b11111111);
        assert_eq!(0b00001111, v.get_block(2));
    }

    #[test]
    fn get_bits() {
        let v = VB::block_with_fill(5, 5, 0b01010101);
        assert_eq!(0b10101, v.get_bits(5, 0, 5));
        assert_eq!(0b101, v.get_bits(5, 0, 3));
        assert_eq!(0b010101, v.get_bits(5, 6, 6));
    }

    #[test]
    fn set_bits() {
        let mut v = VB::block_with_fill(5, 10, 0);
        assert_eq!(0, v.get_bits(5,  0, 5));
        assert_eq!(0, v.get_bits(5,  5, 5));
        assert_eq!(0, v.get_bits(5, 10, 5));

        v.set_bits(5,  0, 5, 17);
        v.set_bits(5,  5, 5,  2);
        v.set_bits(5, 10, 5,  8);

        assert_eq!(17, v.get_bits(5, 0, 5));
        assert_eq!( 2, v.get_bits(5, 5, 5));
        assert_eq!( 8, v.get_bits(5, 10, 5));
    }

    #[test]
    fn set_bit() {
        let mut v = VB::block_with_fill(1, 2, 0);
        assert_eq!(16, v.len());

        assert_eq!(false, v.get_bit(0));
        assert_eq!(false, v.get_bit(1));
        assert_eq!(false, v.get_bit(2));
        assert_eq!(false, v.get_bit(3));
        assert_eq!(false, v.get_bit(4));
        assert_eq!(false, v.get_bit(5));

        v.set_bit(1, true);
        v.set_bit(2, true);
        v.set_bit(5, true);

        assert_eq!(false, v.get_bit(0));
        assert_eq!(true, v.get_bit(1));
        assert_eq!(true, v.get_bit(2));
        assert_eq!(false, v.get_bit(3));
        assert_eq!(false, v.get_bit(4));
        assert_eq!(true, v.get_bit(5));
    }

    #[test]
    fn push_block() {
        let mut v = VB::new();
        v.push_block(6, 0b11111111);
        assert_eq!(0b00111111, v.get_block(0));
        assert_eq!(1, v.len());

        v.push_block(6, 0b11111111);
        assert_eq!(0b00001111, v.get_block(1));
        assert_eq!(2, v.len());

        v.push_block(6, 0b11111111);
        assert_eq!(0b11111111, v.get_block(2));
        assert_eq!(4, v.len());
    }

    #[test]
    fn pop_block_after_push() {
        let mut v = VB::new();
        v.push_block(6, 0b11111111);
        v.push_block(6, 0b11111111);
        v.push_block(6, 0b11111111);
        assert_eq!(Some(0b11111111), v.pop_block(6));
        assert_eq!(Some(0b00001111), v.pop_block(6));
        assert_eq!(Some(0b00111111), v.pop_block(6));
        assert_eq!(None, v.pop_block(6));
    }

    #[test]
    fn pop_block_after_fill() {
        let mut v = VB::block_with_fill(6, 3, 0b11111111);
        assert_eq!(0b11111111, v.get_block(0));
        assert_eq!(0b11111111, v.get_block(1));
        assert_eq!(0b11111111, v.get_block(2));
        assert_eq!(Some(0b11111111), v.pop_block(6));
        assert_eq!(Some(0b00001111), v.pop_block(6));
        assert_eq!(Some(0b00111111), v.pop_block(6));
        assert_eq!(None, v.pop_block(6));
    }

    #[test]
    fn push_bits() {
        let mut v = VB::new();
        v.push_bits(6, 0b100110);
        v.push_bits(6, 0b010100);
        v.push_bits(6, 0b001111);

        assert_eq!(0b00100110, v.get_block(0));
        assert_eq!(0b11110101, v.get_block(1));
        assert_eq!(0b00000000, v.get_block(2));
    }

    #[test]
    fn pop_bits() {
        let mut v = VB::new();
        v.push_bits(6, 0b100110);
        v.push_bits(6, 0b010100);
        v.push_bits(6, 0b001111);

        assert_eq!(Some(0b001111), v.pop_bits(6));
        assert_eq!(0b00000101, v.get_block(1));
        assert_eq!(Some(0b010100), v.pop_bits(6));
        assert_eq!(0b00100110, v.get_block(0));
        assert_eq!(Some(0b100110), v.pop_bits(6));
        assert_eq!(None, v.pop_bits(6));
    }

    #[test]
    fn push_bit() {
        let mut v = VB::new();

        v.push_bit(false);
        v.push_bit(false);
        v.push_bit(true);
        assert_eq!(3, v.len());
        assert_eq!(1, v.block_len());
        v.push_bit(false);
        v.push_bit(true);
        v.push_bit(true);
        assert_eq!(0b00110100, v.get_block(0));

        v.push_bit(true);
        v.push_bit(false);
        assert_eq!(8, v.len());
        assert_eq!(1, v.block_len());
        v.push_bit(true);
        assert_eq!(9, v.len());
        assert_eq!(2, v.block_len());
        assert_eq!(0b01110100, v.get_block(0));
        assert_eq!(0b00000001, v.get_block(1));
    }

    #[test]
    fn pop_bit() {
        let mut v = VB::block_with_fill(1, 2, 0b01010101);

        assert_eq!(2, v.block_len());
        assert_eq!(16, v.len());

        for _ in 0 .. 8 {
            assert_eq!(Some(false), v.pop_bit());
            assert_eq!(Some(true), v.pop_bit());
        }

        assert_eq!(None, v.pop_bit());

        assert_eq!(0, v.block_len());
        assert_eq!(0, v.len());
    }

    #[test]
    fn block_truncate() {
        let mut v = VB::new();
        v.push_bits(5, 17);
        v.push_bits(5, 30);
        v.push_bits(5, 4);
        assert_eq!(3, v.len());
        assert_eq!(2, v.block_len());

        v.block_truncate(5, 1);
        assert_eq!(1, v.len());
        assert_eq!(1, v.block_len());
        assert_eq!(Some(17), v.pop_bits(5));
    }

    #[test]
    fn truncate() {
        let mut v = VB::new();
        v.push_bits(5, 0b10001);
        v.push_bits(5, 0b11110);
        v.push_bits(5, 0b00100);

        v.truncate(5, 2);
        assert_eq!(2, v.len());
        assert_eq!(2, v.block_len());
        assert_eq!(0b10001, v.get_bits(5, 0, 5));
        assert_eq!(0b11110, v.get_bits(5, 5, 5));
        assert_eq!(0b11010001, v.get_block(0));
        assert_eq!(0b00000011, v.get_block(1));

        v.truncate(5, 1);
        assert_eq!(1, v.len());
        assert_eq!(1, v.block_len());
        assert_eq!(0b10001, v.get_bits(5, 0, 5));
        assert_eq!(0b00010001, v.get_block(0));

        v.truncate(5, 2);
    }

    #[test]
    fn shrink_to_fit() {
        let mut v = VB::new();
        for i in 0 .. 5 {
            v.push_bits(5, i);
        }
        v.shrink_to_fit();
        assert_eq!(4, v.block_capacity());
    }

    #[test]
    fn block_resize() {
        let mut v = VB::new();
        v.push_bits(5, 0b11010);
        v.block_resize(5, 3, 0b11111111);
        assert_eq!(0b11010, v.get_bits(5, 0, 5));
        assert_eq!(0b11000, v.get_bits(5, 5, 5));
        assert_eq!(0b11111, v.get_bits(5, 10, 5));

        v.block_resize(5, 1, 0b11111111);
        assert_eq!(1, v.block_len());
        assert_eq!(1, v.len());
        assert_eq!(0b00011010, v.get_block(0));
    }

    #[test]
    fn resize() {
        let mut v = VB::new();
        v.push_bits(5, 0b11010);
        assert_eq!(1, v.len());
        assert_eq!(0b00011010, v.get_block(0));

        v.resize(5, 3, 0b01010);
        assert_eq!(3, v.len());
        assert_eq!(0b11010, v.get_bits(5, 0, 5));
        assert_eq!(0b01010, v.get_bits(5, 5, 5));
        assert_eq!(0b01010, v.get_bits(5, 10, 5));
        assert_eq!(0b01011010, v.get_block(0));
        assert_eq!(0b00101001, v.get_block(1));

        v.resize(5, 1, 0b01010);
        assert_eq!(1, v.block_len());
        assert_eq!(1, v.len());
        assert_eq!(0b00011010, v.get_block(0));
    }

    #[test] #[should_panic]
    fn with_capacity_overflow() {
        VB::with_capacity(5, !0);
    }

    #[test] #[should_panic]
    fn get_block_oob() {
        let v = VB::new();
        v.get_block(0);
    }

    #[test] #[should_panic]
    fn set_block_oob() {
        let mut v = VB::block_with_fill(5, 2, 0);
        v.set_block(5, 2, 0);
    }

    #[test] #[should_panic]
    fn get_bits_oob1() {
        let mut v = VB::new();
        v.push_bits(5, 0);
        v.get_bits(5, 5, 5);
    }

    #[test] #[should_panic]
    fn get_bits_oob2() {
        let v = VB::with_fill(5, 2, 0);
        v.get_bits(5, 6, 5);
    }

    #[test] #[should_panic]
    fn set_bits_oob() {
        let mut v = VB::with_fill(5, 2, 0);
        v.set_bits(5, 10, 5, 0);
    }

    #[test] #[should_panic]
    fn get_bit_oob() {
        let v = VB::with_fill(1, 6, 0);
        v.get_bit(6);
    }

    #[test] #[should_panic]
    fn set_bit_oob() {
        let mut v = VB::with_fill(1, 5, 0);
        v.set_bit(6, true);
    }

    #[test] #[should_panic]
    fn reserve_overflow() {
        let mut v = VB::new();
        v.reserve(5, !0)
    }
}

use std::cmp::Ordering;
use std::fmt;

use space_usage::SpaceUsage;
use storage::{Address, BlockType};
use super::traits::*;

/// A bit vector implementation.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BitVec<Block: BlockType = usize> {
    data: Vec<Block>,
    len:  u64,
}

impl<Block: BlockType> BitVec<Block> {
    /// BitVec maintains the following invariants:
    ///
    ///  1) The size of the underlying vector is tight.
    ///  2) Any extra bits in the last block are zeroed.
    ///
    /// This function checks (asserts) 1 and restores 2. It is used when
    /// the bit vector shrinks.
    #[inline]
    fn restore_invariant(&mut self) {
        // This part of the invariant should not have to be restored:
        debug_assert!(self.data.len() == self.block_len());

        // Mask out any nasty trailing bits:
        let bit_len = self.bit_len();
        self.data.last_mut().map(|block| {
            let mask = Block::low_mask(Block::last_block_bits(bit_len));
            *block = *block & mask;
        });
    }

    /// Creates a new, empty bit vector.
    pub fn new() -> Self {
        BitVec {
            data: Vec::new(),
            len: 0,
        }
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
        let block_capacity = Block::ceil_div_nbits_checked(capacity)
                                 .expect("BitVec::with_capacity: overflow");
        Self::with_block_capacity(block_capacity)
    }

    /// Creates a new, empty bit vector with space allocated for `capacity`
    /// blocks.
    pub fn with_block_capacity(capacity: usize) -> Self {
        BitVec {
            data: Vec::with_capacity(capacity),
            len: 0,
        }
    }

    /// Creates a new bit vector of `len` bits initialized to `value`.
    ///
    /// # Panics
    ///
    /// Panics if `len` is too large. In particular the number of
    /// blocks required by the capacity (`len / Block::nbits()`)
    /// must fit in a `usize`.
    pub fn with_fill(len: u64, value: bool) -> Self {
        let block_size = Block::ceil_div_nbits_checked(len)
                             .expect("BitVec::with_fill: overflow");
        let block_value = if value {!Block::zero()} else {Block::zero()};
        let mut result = Self::with_fill_block(block_size, block_value);
        result.len = len;
        result.restore_invariant();
        result
    }

    /// Creates a new bit vector of `block_len` blocks initialized to `value`.
    pub fn with_fill_block(block_len: usize, value: Block) -> Self {
        BitVec {
            data: vec![ value; block_len ],
            len: Block::mul_nbits(block_len),
        }
    }

    /// How many bits the bit vector can hold without reallocating.
    pub fn capacity(&self) -> u64 {
        Block::mul_nbits(self.block_capacity())
    }

    /// How many blocks the bit vector can hold without reallocating.
    pub fn block_capacity(&self) -> usize {
        self.data.capacity()
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
        let new_block_len = Block::ceil_div_nbits_checked(new_len)
                                .expect("BitVec::resize: overflow");

        if new_len < self.bit_len() || !value {
            self.resize_block(new_block_len, Block::zero());
        } else {
            let trailing = Block::last_block_bits(self.bit_len());
            let remaining = Block::nbits() - trailing;
            self.data.last_mut().map(|block| {
                *block = *block | (Block::low_mask(remaining) << trailing);
            });
            self.resize_block(new_block_len, !Block::zero());
        }

        self.len = new_len;
        self.restore_invariant();
    }

    /// Resizes the bit vector to the given number of blocks,
    /// filling if necessary.
    pub fn resize_block(&mut self, new_len: usize, value: Block) {
        self.data.resize(new_len, value);
        self.len = Block::mul_nbits(new_len);
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
        let intended_cap = self.bit_len() + additional;
        let intended_blocks = Block::ceil_div_nbits_checked(intended_cap)
                                  .expect("BitVec::reserve: overflow");
        let additional_blocks = intended_blocks - self.block_len();
        self.reserve_block(additional_blocks);
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
    pub fn reserve_block(&mut self, additional: usize) {
        self.data.reserve(additional);
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
        let intended_cap = self.bit_len() + additional;
        let intended_blocks = Block::ceil_div_nbits_checked(intended_cap)
                                  .expect("BitVec::reserve: overflow");
        let additional_blocks = intended_blocks - self.block_len();
        self.reserve_exact_block(additional_blocks);
    }

    /// Reserves capacity for at least `additional` more blocks of bits to be
    /// inserted.
    ///
    /// Unlike [`reserve`](#method.reserve), does nothing if the
    /// capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the number of blocks overflows a `usize`.
    pub fn reserve_exact_block(&mut self, additional: usize) {
        self.data.reserve_exact(additional);
    }

    /// Shrinks the capacity to just fit the number of elements.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit()
    }

    /// Shrinks to the given size.
    ///
    /// Does nothing if `len` is greater than the current size.
    pub fn truncate(&mut self, len: u64) {
        let block_len = Block::ceil_div_nbits_checked(len)
                            .expect("BitVec::truncate: overflow");
        self.truncate_block(block_len);
        self.len = len;
        self.restore_invariant();
    }

    /// Shrinks to the given size in blocks.
    ///
    /// Does nothing if `block_len` is greater than the current size in blocks.
    pub fn truncate_block(&mut self, block_len: usize) {
        if block_len < self.block_len() {
            self.data.truncate(block_len);
            self.len = Block::mul_nbits(block_len);
        }
    }

    /// Sets the size to 0 while retaining the allocated storage.
    pub fn clear(&mut self) {
        self.data.clear();
        self.len = 0;
    }
}

impl<Block: BlockType> Bits for BitVec<Block> {
    type Block = Block;

    #[inline]
    fn bit_len(&self) -> u64 {
        self.len
    }

    fn get_bit(&self, index: u64) -> bool {
        assert!(index < self.len, "BitVec:get_bit: out of bounds");

        let address = Address::new::<Block>(index);

        // We don’t need to worry about overflow because we do a bounds
        // check above, and it shouldn’t be possible to create an BitVec
        // that is too large to index.
        self.data[address.block_index].get_bit(address.bit_offset)
    }

    #[inline]
    fn get_block(&self, index: usize) -> Block {
        assert!(index < self.block_len(),
                "BitVec::get_block: out of bounds");
        self.data[index]
    }
}

impl<Block: BlockType> BitsMut for BitVec<Block> {
    fn set_bit(&mut self, index: u64, value: bool) {
        assert!(index < self.len, "BitVec:get_bit: out of bounds");

        let address = Address::new::<Block>(index);

        let old_block = self.data[address.block_index];
        let new_block = old_block.with_bit(address.bit_offset, value);
        self.data[address.block_index] = new_block;
    }

    #[inline]
    fn set_block(&mut self, index: usize, value: Block) {
        match (index + 1).cmp(&self.block_len()) {
            Ordering::Less => {
                self.data[index] = value;
            },
            Ordering::Equal => {
                let mask = Block::low_mask(Block::last_block_bits(self.len));
                self.data[index] = value & mask;
            },
            Ordering::Greater => {
                panic!("BitVec::set_block: out of bounds");
            },
        }
    }
}

impl<Block: BlockType> BitVector for BitVec<Block> {
    fn push_bit(&mut self, value: bool) {
        let capacity = Block::nbits() as u64 * self.data.len() as u64;
        if self.len == capacity {
            self.data.push(Block::zero());
        }

        let old_len = self.len;
        self.len = old_len + 1;
        self.set_bit(old_len, value);
    }

    fn pop_bit(&mut self) -> Option<bool> {
        if self.len == 0 { return None; }

        let result = Some(self.get_bit(self.len - 1));
        self.len -= 1;

        if Block::mod_nbits(self.len) == 0 {
            self.data.pop();
        } else {
            self.restore_invariant();
        }

        result
    }

    fn push_block(&mut self, value: Block) {
        let block_len = self.block_len();
        self.len = Block::nbits() as u64 * (block_len as u64 + 1);

        if self.data.len() < block_len + 1 {
            self.data.push(value);
        } else {
            self.set_block(block_len, value);
        }
    }
}

impl<Block: BlockType> fmt::Binary for BitVec<Block> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        for i in 0 .. self.bit_len() {
            let bit = if self.get_bit(i) {"1"} else {"0"};
            try!(formatter.write_str(bit));
        }

        Ok(())
    }
}

impl<Block: BlockType> SpaceUsage for BitVec<Block> {
    fn is_stack_only() -> bool { false }

    fn heap_bytes(&self) -> usize {
        self.data.heap_bytes()
    }
}

impl<Block: BlockType> Default for BitVec<Block> {
    fn default() -> Self {
        BitVec::new()
    }
}

#[cfg(test)]
mod test {
    use bit_vector::*;

    macro_rules! assert_bv {
        ($expected:expr, $actual:expr) => {
            assert_eq!($expected, format!("{:b}", $actual))
        }
    }

    #[test]
    fn new() {
        let bv: BitVec = BitVec::new();
        assert_eq!(0, bv.bit_len());
        assert_eq!(0, bv.block_len());
    }

    #[test]
    fn capacity() {
        let bv: BitVec<u32> = BitVec::new();
        assert_eq!(0, bv.capacity());

        let bv: BitVec<u32> = BitVec::with_capacity(65);
        assert_eq!(96, bv.capacity());
    }

    #[test]
    fn push_binary() {
        let mut bv: BitVec = BitVec::new();
        bv.push_bit(true);
        bv.push_bit(false);
        bv.push_bit(false);
        assert_eq!("100", format!("{:b}", bv));
    }

    #[test]
    fn fill_with_block() {
        let bv: BitVec<u8> = BitVec::with_fill_block(3, 0b101);
        assert_eq!(3, bv.block_capacity());
        assert_bv!("101000001010000010100000", bv);
    }

    #[test]
    fn fill_with() {
        let bv0: BitVec = BitVec::with_fill(20, false);
        let bv1: BitVec = BitVec::with_fill(20, true);

        assert_eq!(false, bv0.get_bit(3));
        assert_eq!(true, bv1.get_bit(3));

        assert_bv!("00000000000000000000", bv0);
        assert_bv!("11111111111111111111", bv1);
    }

    #[test]
    fn push_pop() {
        let mut bv: BitVec = BitVec::new();
        bv.push_bit(true);
        bv.push_bit(false);
        bv.push_bit(false);
        assert_eq!(Some(false), bv.pop_bit());
        assert_eq!(Some(false), bv.pop_bit());
        assert_eq!(Some(true), bv.pop_bit());
        assert_eq!(None, bv.pop_bit());
    }

    #[test]
    fn push_get() {
        let mut bv: BitVec = BitVec::new();
        bv.push_bit(true);
        bv.push_bit(false);
        bv.push_bit(false);
        assert_eq!(3, bv.bit_len());
        assert_eq!(1, bv.block_len());
        assert_eq!(true, bv.get_bit(0));
        assert_eq!(false, bv.get_bit(1));
        assert_eq!(false, bv.get_bit(2));
    }

    #[test]
    #[should_panic]
    fn get_oob() {
        let mut bv: BitVec = BitVec::new();
        bv.push_bit(true);
        bv.get_bit(3);
    }

    #[test]
    fn push_block() {
        let mut bv: BitVec<u32> = BitVec::new();
        bv.push_block(0);
        assert_bv!("00000000000000000000000000000000", bv);
    }

    #[test]
    fn push_bits_get_block() {
        let mut bv: BitVec = BitVec::new();
        bv.push_bit(true);  // 1
        bv.push_bit(true);  // 2
        bv.push_bit(false); // (4)
        bv.push_bit(false); // (8)
        bv.push_bit(true);  // 16

        assert_eq!(19, bv.get_block(0));
    }

    #[test]
    fn push_block_get_block() {
        let mut bv: BitVec = BitVec::new();
        bv.push_block(358);
        bv.push_block(!0);
        assert_eq!(358, bv.get_block(0));
        assert_eq!(!0, bv.get_block(1));
    }

    #[test]
    #[should_panic]
    fn get_block_oob() {
        let mut bv: BitVec = BitVec::new();
        bv.push_bit(true);
        bv.get_block(3);
    }

    #[test]
    fn push_block_get_bit() {
        let mut bv: BitVec = BitVec::new();
        bv.push_block(0b10101);
        assert_eq!(true, bv.get_bit(0));
        assert_eq!(false, bv.get_bit(1));
        assert_eq!(true, bv.get_bit(2));
        assert_eq!(false, bv.get_bit(3));
        assert_eq!(true, bv.get_bit(4));
        assert_eq!(false, bv.get_bit(5));
    }

    #[test]
    fn push_block_set_get() {
        let mut bv: BitVec = BitVec::new();
        bv.push_block(0);
        bv.set_bit(0, true);
        bv.set_bit(1, true);
        bv.set_bit(2, false);
        bv.set_bit(3, true);
        bv.set_bit(4, false);
        assert_eq!(true, bv.get_bit(0));
        assert_eq!(true, bv.get_bit(1));
        assert_eq!(false, bv.get_bit(2));
        assert_eq!(true, bv.get_bit(3));
        assert_eq!(false, bv.get_bit(4));
    }

    #[test]
    fn set_block_mask() {
        let mut bv: BitVec = BitVec::new();

        bv.push_bit(false);
        bv.set_block(0, 0b11);
        assert_eq!(0b01, bv.get_block(0));

        bv.push_bit(false);
        bv.set_block(0, 0b11);
        assert_eq!(0b11, bv.get_block(0));
    }

    #[test]
    fn resize() {
        let mut bv: BitVec<u8> = BitVec::new();

        bv.push_bit(true);
        bv.push_bit(false);
        bv.push_bit(true);
        assert_bv!("101", bv);

        bv.resize(21, false);
        assert_bv!("101000000000000000000", bv);

        bv.resize(22, false);
        assert_bv!("1010000000000000000000", bv);

        bv.resize(5, false);
        assert_bv!("10100", bv);

        bv.resize(21, true);
        assert_bv!("101001111111111111111", bv);

        bv.resize(4, true);
        assert_bv!("1010", bv);

        bv.push_block(0b11111111);
        assert_bv!("1010000011111111", bv);
    }

    #[test]
    fn resize_block() {
        let mut bv: BitVec<u8> = BitVec::new();

        bv.push_bit(true);
        bv.push_bit(false);
        bv.push_bit(true);
        assert_bv!("101", bv);

        bv.resize_block(1, 0);
        assert_bv!("10100000", bv);

        bv.resize_block(3, 0b01000101);
        assert_bv!("101000001010001010100010", bv);

        bv.resize_block(2, 0);
        assert_bv!("1010000010100010", bv);
    }
}

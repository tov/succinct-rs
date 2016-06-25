use num::ToPrimitive;

use storage::BlockType;
use super::traits::*;

/// A bit vector implementation.
#[derive(Clone, Debug)]
pub struct BitVec<Block: BlockType = usize> {
    data: Vec<Block>,
    len:  u64,
}

impl<Block: BlockType> BitVec<Block> {
    /// Creates a new, empty bit vector.
    pub fn new() -> Self {
        BitVec {
            data: Vec::new(),
            len: 0,
        }
    }

    /// Creates a new, empty bit vector with space allocated for `capacity`
    /// bits.
    pub fn with_capacity(capacity: u64) -> Self {
        let block_capacity = capacity.ceil_div(Block::nbits() as u64)
                                     .to_usize()
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
}

impl<Block: BlockType> Bits for BitVec<Block> {
    type Block = Block;

    fn bit_len(&self) -> u64 {
        self.len
    }

    fn get_bit(&self, index: u64) -> bool {
        assert!(index < self.len, "BitVec:get_bit: out of bounds");

        let block_index = (index / Block::nbits() as u64) as usize;
        let bit_offset = (index % Block::nbits() as u64) as usize;

        // We don’t need to worry about overflow because we do a bounds
        // check above, and it shouldn’t be possible to create an IntVec
        // that is too large to index.
        self.data[block_index].get_bit(bit_offset)
    }
}

impl<Block: BlockType> BitsMut for BitVec<Block> {
    fn set_bit(&mut self, index: u64, value: bool) {
        assert!(index < self.len, "BitVec:get_bit: out of bounds");

        let block_index = (index / Block::nbits() as u64) as usize;
        let bit_offset = (index % Block::nbits() as u64) as usize;

        let old_block = self.data[block_index];
        let new_block = old_block.with_bit(bit_offset, value);
        self.data[block_index] = new_block;
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
        result
    }
}

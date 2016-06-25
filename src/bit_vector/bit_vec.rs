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

    #[inline]
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

        let block_index = (index / Block::nbits() as u64) as usize;
        let bit_offset = (index % Block::nbits() as u64) as usize;

        let old_block = self.data[block_index];
        let new_block = old_block.with_bit(bit_offset, value);
        self.data[block_index] = new_block;
    }

    #[inline]
    fn set_block(&mut self, index: usize, value: Block) {
        assert!(index < self.block_len(),
                "BitVec::set_block: out of bounds");
        self.data[index] = value;
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

    fn push_block(&mut self, value: Block) {
        let block_len = self.block_len();

        // Zero out any trailing bits
        let keep = self.len % Block::nbits() as u64;
        if keep > 0 {
            let mask = Block::low_mask(keep as usize);
            self.data[block_len - 1] = self.data[block_len - 1] & mask;
        }

        // Expand the length and set the new last block
        self.len = Block::nbits() as u64 * (block_len as u64 + 1);

        if self.data.len() < block_len + 1 {
            self.data.push(value);
        } else {
            self.set_block(block_len, value);
        }
    }
}

#[cfg(test)]
mod test {
    use bit_vector::*;

    #[test]
    fn new() {
        let bv: BitVec = BitVec::new();
        assert_eq!(0, bv.bit_len());
        assert_eq!(0, bv.block_len());
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

        assert_eq!(32, bv.bit_len());
        assert_eq!(1, bv.block_len());
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
}

use num::{One, Zero};

use storage::BlockType;

/// Interface for read-only bit vector operations.
pub trait BitSlice {
    /// The underlying block type used to store the bits of the slice.
    type Block: BlockType;

    /// The length of the slice in bits.
    fn bit_len(&self) -> u64;

    /// Gets the bit at `position`
    ///
    /// # Panics
    ///
    /// Panics if `position` is out of bounds.
    fn get_bit(&self, position: u64) -> bool;

    /// The length of the slice in blocks.
    fn block_len(&self) -> usize {
        self.bit_len().ceil_div(Self::Block::nbits() as u64) as usize
    }

    /// Gets the block at `position`
    ///
    /// The bits are laid out `Block::nbits()` per block, with the notional
    /// zeroth bit in the least significant position. If `self.bit_len()` is
    /// not a multiple of `Block::nbits()` then the last block will
    /// contain extra bits that are not part of the bit vector.
    ///
    /// The default implementation assembles a block by reading each of its
    /// bits. Consider it a slow reference implementation, and override it.
    ///
    /// # Panics
    ///
    /// Panics if `position` is out of bounds.
    fn get_block(&self, position: usize) -> Self::Block {
        assert!(position < self.block_len(),
                "IntSlice::get_block: out of bounds");

        let mut result = Self::Block::zero();
        let bit_position = position as u64 * Self::Block::nbits() as u64;

        for i in 0 .. Self::Block::nbits() as u64 {
            result = result << 1;
            if bit_position + i < self.bit_len() && self.get_bit(bit_position + i) {
                result = result | Self::Block::one();
            }
        }

        result
    }

    /// Gets `count` bits starting at bit index `start`, interpreted as a
    /// little-endian integer.
    ///
    /// # Panics
    ///
    /// Panics if the bit span goes out of bounds.
    fn get_bits(&self, start: u64, count: usize) -> Self::Block {
        let limit = start + count as u64;
        assert!(limit <= self.bit_len(), "BitSlice::get_bits: out of bounds");

        let block_bits = Self::Block::nbits();
        let block_index = (start / block_bits as u64) as usize;
        let bit_offset = (start % block_bits as u64) as usize;

        let margin = block_bits - bit_offset;

        if margin >= count {
            let block = self.get_block(block_index);
            return block.get_bits(bit_offset, count)
        }

        let extra = count - margin;

        let block1 = self.get_block(block_index);
        let block2 = self.get_block(block_index + 1);

        let high_bits = block1.get_bits(bit_offset, margin);
        let low_bits = block2.get_bits(0, extra);

        (high_bits << extra) | low_bits
    }
}

/// Interface for mutable bit vector operations that don’t affect the
/// length.
pub trait BitSliceMut: BitSlice {
    /// Sets the bit at `position` to `value`.
    ///
    /// # Panics
    ///
    /// Panics if `position` is out of bounds.
    fn set_bit(&mut self, position: u64, value: bool);

    /// Sets the block at `position` to `value`.
    ///
    /// The bits are laid out `Block::nbits()` per block, with the notional
    /// zeroth bit in the least significant position. If `self.bit_len()` is
    /// not a multiple of `Block::nbits()` then the last block will
    /// contain extra bits that are not part of the bit vector. The values of
    /// these trailing bits are unspecified, and setting them should not be
    /// relied upon.
    ///
    /// The default implementation sets a block by setting each of its bits
    /// in turn. Consider it a slow reference implementation, and override it.
    ///
    /// # Panics
    ///
    /// Panics if `position` is out of bounds.
    fn set_block(&mut self, position: usize, mut value: Self::Block) {
        let bit_position = position as u64 * Self::Block::nbits() as u64;

        for i in 0 .. Self::Block::nbits() as u64 {
            let bit = value & Self::Block::one() != Self::Block::zero();
            self.set_bit(bit_position + i, bit);
            value = value >> 1;
        }
    }

    /// Sets `count` bits starting at bit index `start`, interpreted as a
    /// little-endian integer.
    ///
    /// # Panics
    ///
    /// Panics if the bit span goes out of bounds.
    fn set_bits(&mut self, start: u64, count: usize, value: Self::Block) {
        let limit = start + count as u64;
        assert!(limit <= self.bit_len(), "BitSlice::get_bits: out of bounds");

        let block_bits = Self::Block::nbits();
        let block_index = (start / block_bits as u64) as usize;
        let bit_offset = (start % block_bits as u64) as usize;

        let margin = block_bits - bit_offset;

        if margin >= count {
            let old_block = self.get_block(block_index);
            let new_block = old_block.set_bits(bit_offset, count, value);
            self.set_block(block_index, new_block);
            return;
        }

        let extra = count - margin;

        let old_block1 = self.get_block(block_index);
        let old_block2 = self.get_block(block_index + 1);

        let high_bits = value >> extra;

        let new_block1 = old_block1.set_bits(bit_offset, margin, high_bits);
        let new_block2 = old_block2.set_bits(0, extra, value);

        self.set_block(block_index, new_block1);
        self.set_block(block_index + 1, new_block2);
    }
}

/// Interface for full bit vector operations.
pub trait BitVector: BitSliceMut {
    /// Adds the given bit to the end of the bit vector.
    fn push_bit(&mut self, value: bool);

    /// Removes and returns the last bit, if any.
    fn pop_bit(&mut self) -> Option<bool>;

    /// Pushes `value` 0 or more times until the size of the bit
    /// vector is block-aligned.
    fn align_block(&mut self, value: bool) {
        while self.bit_len() % Self::Block::nbits() as u64 != 0 {
            self.push_bit(value);
        }
    }

    /// Pushes the given block onto the end of the bit vector.
    ///
    /// If the end of the bit vector is not currently block-aligned,
    /// it pads with 0s up to the next block before pushing.
    ///
    /// The default implementation pushes the block one bit at a time;
    /// override it with something more efficient.
    fn push_block(&mut self, mut value: Self::Block) {
        self.align_block(false);

        for _ in 0 .. Self::Block::nbits() {
            self.push_bit(value & Self::Block::one() != Self::Block::zero());
            value = value >> 1;
        }
    }
}

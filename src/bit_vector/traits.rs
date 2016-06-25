use num::{One, Zero};

use storage::BlockType;

/// Interface for read-only bit vector operations.
///
/// Minimal complete definition is `get_bit` or `get_block`, since each
/// is defined in terms of the other. Note that `get_block` in terms of
/// `get_bit` is inefficient, and thus you should implement `get_block`
/// directly if possible.
pub trait Bits {
    /// The underlying block type used to store the bits of the slice.
    type Block: BlockType;

    /// The length of the slice in bits.
    fn bit_len(&self) -> u64;

    /// Gets the bit at `position`
    ///
    /// The default implementation calls `get_block` and masks out the
    /// correct bit.
    ///
    /// # Panics
    ///
    /// Panics if `position` is out of bounds.
    fn get_bit(&self, position: u64) -> bool {
        assert!(position < self.bit_len(), "Bits::get_bit: out of bounds");

        let address = Address::new::<Self::Block>(position);
        let block = self.get_block(address.block_index);
        block.get_bit(address.bit_offset)
    }

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
        assert!(limit <= self.bit_len(), "Bits::get_bits: out of bounds");

        let address = Address::new::<Self::Block>(start);
        let margin = Self::Block::nbits() - address.bit_offset;

        if margin >= count {
            let block = self.get_block(address.block_index);
            return block.get_bits(address.bit_offset, count)
        }

        let extra = count - margin;

        let block1 = self.get_block(address.block_index);
        let block2 = self.get_block(address.block_index + 1);

        let high_bits = block1.get_bits(address.bit_offset, margin);
        let low_bits = block2.get_bits(0, extra);

        (high_bits << extra) | low_bits
    }
}

/// Interface for mutable bit vector operations that don’t affect the
/// length.
///
/// Minimal complete definition is `set_bit` or `set_block`, since each
/// is defined in terms of the other. Note that `set_block` in terms of
/// `set_bit` is inefficient, and thus you should implement `get_block`
/// directly if possible.
pub trait BitsMut: Bits {
    /// Sets the bit at `position` to `value`.
    ///
    /// The default implementation uses `get_block` and `set_block`.
    ///
    /// # Panics
    ///
    /// Panics if `position` is out of bounds.
    fn set_bit(&mut self, position: u64, value: bool) {
        assert!(position < self.bit_len(), "BitsMut::set_bit: out of bounds");

        let address = Address::new::<Self::Block>(position);
        let old_block = self.get_block(address.block_index);
        let new_block = old_block.with_bit(address.bit_offset, value);
        self.set_block(address.block_index, new_block);
    }

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
        assert!(limit <= self.bit_len(), "BitsMut::set_bits: out of bounds");

        let address = Address::new::<Self::Block>(start);
        let margin = Self::Block::nbits() - address.bit_offset;

        if margin >= count {
            let old_block = self.get_block(address.block_index);
            let new_block = old_block.with_bits(address.bit_offset, count, value);
            self.set_block(address.block_index, new_block);
            return;
        }

        let extra = count - margin;

        let old_block1 = self.get_block(address.block_index);
        let old_block2 = self.get_block(address.block_index + 1);

        let high_bits = value >> extra;

        let new_block1 = old_block1.with_bits(address.bit_offset,
                                              margin, high_bits);
        let new_block2 = old_block2.with_bits(0, extra, value);

        self.set_block(address.block_index, new_block1);
        self.set_block(address.block_index + 1, new_block2);
    }
}

/// Interface for full bit vector operations.
pub trait BitVector: BitsMut {
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

/// The address of a bit, as an index to a block and the index of a bit
/// in that block.
#[derive(Clone, Copy, Debug)]
struct Address {
    block_index: usize,
    bit_offset: usize,
}

impl Address {
    fn new<Block: BlockType>(bit_index: u64) -> Self {
        let block_bits = Block::nbits() as u64;
        Address {
            block_index: (bit_index / block_bits) as usize,
            bit_offset: (bit_index % block_bits) as usize,
        }
    }
}

impl<Block: BlockType> Bits for [Block] {
    type Block = Block;

    #[inline]
    fn bit_len(&self) -> u64 {
        self.len() as u64 * Block::nbits() as u64
    }

    #[inline]
    fn block_len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn get_block(&self, position: usize) -> Block {
        self[position]
    }
}

impl<Block: BlockType> BitsMut for [Block] {
    #[inline]
    fn set_block(&mut self, position: usize, value: Block) {
        self[position] = value;
    }
}

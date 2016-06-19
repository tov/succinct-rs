//! Traits for describing how bits and arrays of bits are stored.

use std::io;
use std::mem;

use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use num::{PrimInt, ToPrimitive};

/// Types that can be used for `IntVec` storage.
pub trait BlockType: PrimInt {
    /// The number of bits in a block.
    #[inline]
    fn nbits() -> usize {
        8 * mem::size_of::<Self>()
    }

    /// The bit mask consisting of `Self::nbits() - element_bits` zeroes
    /// followed by `element_bits` ones.
    ///
    /// # Precondition
    ///
    /// `element_bits <= Self::nbits()`
    #[inline]
    fn low_mask(element_bits: usize) -> Self {
        debug_assert!(element_bits <= Self::nbits());

        if element_bits == Self::nbits() {
            !Self::zero()
        } else {
            (Self::one() << element_bits) - Self::one()
        }
    }

    /// The bit mask with the `bit_index`th bit set.
    ///
    /// Bits are index in little-endian style based at 0.
    ///
    /// # Precondition
    ///
    /// `bit_index < Self::nbits()`
    #[inline]
    fn nth_mask(bit_index: usize) -> Self {
        Self::one() << bit_index
    }

    /// Extracts `len` bits starting at bit offset `start`.
    ///
    /// # Precondition
    ///
    /// `start + len < Self::nbits()`
    #[inline]
    fn get_bits(self, start: usize, len: usize) -> Self {
        if len == 0 { return Self::zero(); }

        let limit      = start + len;
        debug_assert!(limit <= Self::nbits());

        (self >> start) & Self::low_mask(len)
    }

    /// Sets `len` bits to `value` starting at offset `start`.
    ///
    /// # Precondition
    ///
    /// `start + len < Self::nbits()`
    #[inline]
    fn set_bits(self, start: usize, len: usize, value: Self) -> Self {
        if len == 0 { return self; }

        let limit      = start + len;
        debug_assert!(limit <= Self::nbits());

        let mask = Self::low_mask(len) << start;
        let shifted_value = value << start;

        (self & !mask) | (shifted_value & mask)
    }

    /// Extracts the value of the `bit_index`th bit.
    #[inline]
    fn get_bit(self, bit_index: usize) -> bool {
        self & Self::nth_mask(bit_index) != Self::zero()
    }

    /// Sets the value of the `bit_index`th bit to true.
    #[inline]
    fn set_bit(self, bit_index: usize, bit_value: bool) -> Self {
        if bit_value {
            self | Self::nth_mask(bit_index)
        } else {
            self & !Self::nth_mask(bit_index)
        }
    }

    /// Returns the smallest number `n` such that `2.pow(n) >= self`.
    #[inline]
    fn ceil_log2(self) -> usize {
        if self <= Self::one() { return 0; }
        Self::nbits() - (self - Self::one()).leading_zeros() as usize
    }

    /// Returns the largest number `n` such that `2.pow(n) <= self`.
    #[inline]
    fn floor_log2(self) -> usize {
        if self <= Self::one() { return 0; }
        Self::nbits() - 1 - self.leading_zeros() as usize
    }

    /// Returns the smallest number `n` such that `n * divisor >= self`.
    #[inline]
    fn ceil_div(self, divisor: Self) -> Self {
        (self + divisor - Self::one()) / divisor
    }

    /// Returns the total count of ones up through the `index`th digit,
    /// little-endian style.
    fn rank1(self, index: usize) -> usize {
        (self & Self::low_mask(index + 1)).count_ones() as usize
    }

    /// Reads a block with the specified endianness.
    fn read_block<R, T>(source: &mut R) -> io::Result<Self>
        where R: io::Read, T: ByteOrder;

    /// Writes a block with the specified endianness.
    fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
        where W: io::Write, T: ByteOrder;
}


impl BlockType for u8 {
    fn read_block<R, T>(source: &mut R) -> io::Result<Self>
        where R: io::Read,
              T: ByteOrder {
        source.read_u8()
    }

    fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
        where W: io::Write,
              T: ByteOrder {
        sink.write_u8(*self)
    }
}

macro_rules! impl_block_type {
    ($ty:ident, $read:ident, $write:ident)
        =>
    {
        impl BlockType for $ty {
            fn read_block<R, T>(source: &mut R) -> io::Result<Self>
                where R: io::Read,
                      T: ByteOrder {
                source.$read::<T>()
            }

            fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
                where W: io::Write,
                      T: ByteOrder {
                sink.$write::<T>(*self)
            }
        }
    }
}

impl_block_type!(u16, read_u16, write_u16);
impl_block_type!(u32, read_u32, write_u32);
impl_block_type!(u64, read_u64, write_u64);

#[cfg(target_pointer_width = "64")]
impl BlockType for usize {
    fn read_block<R, T>(source: &mut R) -> io::Result<Self>
        where R: io::Read,
              T: ByteOrder {
        source.read_u64::<T>().map(|x| x as usize)
    }

    fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
        where W: io::Write,
              T: ByteOrder {
        sink.write_u64::<T>(*self as u64)
    }
}

#[cfg(target_pointer_width = "32")]
impl BlockType for usize {
    fn read_block<R, T>(source: &mut R) -> io::Result<Self>
        where R: io::Read,
              T: ByteOrder {
        source.read_u32::<T>().map(|x| x as usize)
    }

    fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
        where W: io::Write,
              T: ByteOrder {
        sink.write_u32::<T>(*self as u32)
    }
}

/// Interface for read-only bit vector operations.
pub trait BitStore {
    /// The type of each block of storage.
    type Block: BlockType;

    /// The length of the bit vector in blocks.
    fn block_len(&self) -> usize;

    /// The length of the bit vector in bits.
    ///
    /// Default implementation is `self.block_len() * Block::nbits()`.
    #[inline]
    fn bit_len(&self) -> u64 {
        self.block_len() as u64 * Self::Block::nbits() as u64
    }

    /// Gets the value of the block at `position`
    fn get_block(&self, position: usize) -> Self::Block;

    /// Gets the bit at `position`
    #[inline]
    fn get_bit(&self, position: u64) -> bool {
        assert!(position < self.bit_len(), "BitStore::get: out of bounds");
        let block_bits = Self::Block::nbits() as u64;
        let block_index = (position / block_bits).to_usize().unwrap();
        let bit_offset = (position % block_bits) as usize;
        self.get_block(block_index).get_bit(bit_offset)
    }
}

/// Interface for mutable bit vector operations.
pub trait BitStoreMut : BitStore {
    /// Sets the block at `position` to `value`.
    fn set_block(&mut self, position: usize, value: Self::Block);

    /// Sets the bit at `position` to `value`.
    #[inline]
    fn set_bit(&mut self, position: u64, value: bool) {
        assert!(position < self.bit_len(), "BitStore::set: out of bounds");
        let block_bits = Self::Block::nbits() as u64;
        let block_index = (position / block_bits).to_usize().unwrap();
        let bit_offset = (position % block_bits) as usize;
        let old_block = self.get_block(block_index);
        let new_block = old_block.set_bit(bit_offset, value);
        self.set_block(block_index, new_block);
    }
}

impl<Block: BlockType> BitStore for [Block] {
    type Block = Block;

    #[inline]
    fn block_len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn get_block(&self, position: usize) -> Block {
        self[position]
    }
}

impl<Block: BlockType> BitStoreMut for [Block] {
    #[inline]
    fn set_block(&mut self, position: usize, value: Block) {
        self[position] = value;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn low_mask() {
        assert_eq!(0b00011111, u8::low_mask(5));
        assert_eq!(0b0011111111111111, u16::low_mask(14));
    }

    #[test]
    fn nth_mask() {
        assert_eq!(0b10000000, u8::nth_mask(7));
        assert_eq!(0b01000000, u8::nth_mask(6));
        assert_eq!(0b00100000, u8::nth_mask(5));
        assert_eq!(0b00000010, u8::nth_mask(1));
        assert_eq!(0b00000001, u8::nth_mask(0));
    }

    #[test]
    fn get_bits() {
        assert_eq!(0b0,
                   0b0100110001110000u16.get_bits(0, 0));
        assert_eq!(0b010,
                   0b0100110001110000u16.get_bits(13, 3));
        assert_eq!(    0b110001,
                   0b0100110001110000u16.get_bits(6, 6));
        assert_eq!(           0b10000,
                   0b0100110001110000u16.get_bits(0, 5));
        assert_eq!(0b0100110001110000,
                   0b0100110001110000u16.get_bits(0, 16));
    }

    #[test]
    fn set_bits() {
        assert_eq!(0b0111111111000001,
                   0b0110001111000001u16.set_bits(10, 3, 0b111));
        assert_eq!(0b0101110111000001,
                   0b0110001111000001u16.set_bits(9, 5, 0b01110));
        assert_eq!(0b0110001111000001,
                   0b0110001111000001u16.set_bits(14, 0, 0b01110));
        assert_eq!(0b0110001110101010,
                   0b0110001111000001u16.set_bits(0, 8, 0b10101010));
        assert_eq!(0b0000000000000010,
                   0b0110001111000001u16.set_bits(0, 16, 0b10));
    }

    #[test]
    fn get_bit() {
        assert!(! 0b00000000u8.get_bit(0));
        assert!(! 0b00000000u8.get_bit(1));
        assert!(! 0b00000000u8.get_bit(2));
        assert!(! 0b00000000u8.get_bit(3));
        assert!(! 0b00000000u8.get_bit(7));
        assert!(! 0b10101010u8.get_bit(0));
        assert!(  0b10101010u8.get_bit(1));
        assert!(! 0b10101010u8.get_bit(2));
        assert!(  0b10101010u8.get_bit(3));
        assert!(  0b10101010u8.get_bit(7));
    }

    #[test]
    fn set_bit() {
        assert_eq!(0b00100000, 0b00000000u8.set_bit(5, true));
        assert_eq!(0b00000000, 0b00000000u8.set_bit(5, false));
        assert_eq!(0b10101010, 0b10101010u8.set_bit(7, true));
        assert_eq!(0b00101010, 0b10101010u8.set_bit(7, false));
        assert_eq!(0b10101011, 0b10101010u8.set_bit(0, true));
        assert_eq!(0b10101010, 0b10101010u8.set_bit(0, false));
    }

    #[test]
    fn floor_log2() {
        assert_eq!(0, 1u32.floor_log2());
        assert_eq!(1, 2u32.floor_log2());
        assert_eq!(1, 3u32.floor_log2());
        assert_eq!(2, 4u32.floor_log2());
        assert_eq!(2, 5u32.floor_log2());
        assert_eq!(2, 7u32.floor_log2());
        assert_eq!(3, 8u32.floor_log2());
    }

    #[test]
    fn ceil_log2() {
        assert_eq!(0, 1u32.ceil_log2());
        assert_eq!(1, 2u32.ceil_log2());
        assert_eq!(2, 3u32.ceil_log2());
        assert_eq!(2, 4u32.ceil_log2());
        assert_eq!(3, 5u32.ceil_log2());
        assert_eq!(3, 7u32.ceil_log2());
        assert_eq!(3, 8u32.ceil_log2());
        assert_eq!(4, 9u32.ceil_log2());
    }

    #[test]
    fn ceil_div() {
        assert_eq!(6, 12u32.ceil_div(2));
        assert_eq!(4, 12u32.ceil_div(3));
        assert_eq!(3, 12u32.ceil_div(4));
        assert_eq!(3, 12u32.ceil_div(5));
        assert_eq!(2, 12u32.ceil_div(6));
        assert_eq!(2, 12u32.ceil_div(7));
        assert_eq!(2, 12u32.ceil_div(11));
        assert_eq!(1, 12u32.ceil_div(12));
    }

    #[test]
    fn rank1() {
        assert_eq!(0, 0b00000000u8.rank1(0));
        assert_eq!(0, 0b00000000u8.rank1(7));
        assert_eq!(1, 0b01010101u8.rank1(0));
        assert_eq!(1, 0b01010101u8.rank1(1));
        assert_eq!(2, 0b01010101u8.rank1(2));
        assert_eq!(2, 0b01010101u8.rank1(3));

        assert_eq!(3, 0b00001111u8.rank1(2));
        assert_eq!(4, 0b00001111u8.rank1(3));
        assert_eq!(4, 0b00001111u8.rank1(4));
        assert_eq!(4, 0b00001111u8.rank1(5));
        assert_eq!(4, 0b00001111u8.rank1(7));

        assert_eq!(0, 0b11110000u8.rank1(0));
        assert_eq!(0, 0b11110000u8.rank1(3));
        assert_eq!(1, 0b11110000u8.rank1(4));
        assert_eq!(2, 0b11110000u8.rank1(5));
        assert_eq!(4, 0b11110000u8.rank1(7));
    }

    #[test]
    fn store_bit_len() {
        let v = vec![ 0u32; 4 ];
        assert_eq!(128, v.bit_len());
    }

    #[test]
    fn store_block_len() {
        let v = vec![ 0u32; 4 ];
        assert_eq!(4, v.block_len());
    }

    #[test]
    fn store_set_get_bit() {
        let mut v = vec![ 0b10101010u8; 4 ];
        assert!(! v.get_bit(0));
        assert!(  v.get_bit(1));
        assert!(! v.get_bit(2));
        assert!(  v.get_bit(3));

        v.set_bit(2, true);

        assert!(! v.get_bit(0));
        assert!(  v.get_bit(1));
        assert!(  v.get_bit(2));
        assert!(  v.get_bit(3));
    }
}


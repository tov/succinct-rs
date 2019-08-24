//! Traits describing how bits and arrays of bits are stored.

use std::fmt;
use std::io;
use std::mem;

use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use num_traits::{One, PrimInt, ToPrimitive};

use bit_vec::{BitVec, BitVecMut};
use rank::{BitRankSupport, RankSupport};
use space_usage::SpaceUsage;

/// Types that can be used for `IntVector` and `BitVector` storage.
///
/// This trait is kind of a grab bag of methods right now. It includes:
///
///   - methods for computing sizes and offsets relative to the block size,
///   - methods for getting and setting individual and groups of bits,
///   - a method for computing rank,
///   - three arithmetic methods that probably belong elsewhere, and
///   - block-based, endian-specified I/O.
pub trait BlockType:
    PrimInt + BitVec + BitVecMut + BitRankSupport + RankSupport<Over = bool> + SpaceUsage + fmt::Debug
{
    // Methods for computing sizes and offsets relative to the block size.

    /// The number of bits in a block.
    #[inline]
    fn nbits() -> usize {
        8 * mem::size_of::<Self>()
    }

    /// Returns `index / Self::nbits()`, computed by shifting.
    ///
    /// This is intended for converting a bit address into a block
    /// address, which is why it takes `u64` and returns `usize`.
    /// There is no check that the result actually fits in a `usize`,
    /// so this should only be used when `index` is already known to
    /// be small enough.
    #[inline]
    fn div_nbits(index: u64) -> usize {
        (index >> Self::lg_nbits()) as usize
    }

    /// Returns `index / Self::nbits()`, computed by shifting.
    ///
    /// This is intended for converting a bit address into a block
    /// address, which is why it takes `u64` and returns `usize`.
    #[inline]
    fn checked_div_nbits(index: u64) -> Option<usize> {
        (index >> Self::lg_nbits()).to_usize()
    }

    /// Returns `index / Self::nbits()` rounded up, computed by shifting.
    ///
    /// This is intended for converting a bit size into a block
    /// size, which is why it takes `u64` and returns `usize`.
    #[inline]
    fn ceil_div_nbits(index: u64) -> usize {
        Self::div_nbits(index + (Self::nbits() as u64 - 1))
    }

    /// Returns `index / Self::nbits()` rounded up, computed by shifting.
    ///
    /// This is intended for converting a bit size into a block
    /// size, which is why it takes `u64` and returns `usize`.
    /// There is no check that the result actually fits in a `usize`,
    /// so this should only be used when `index` is already known to
    /// be small enough.
    #[inline]
    fn checked_ceil_div_nbits(index: u64) -> Option<usize> {
        Self::checked_div_nbits(index + (Self::nbits() as u64 - 1))
    }

    /// Returns `index % Self::nbits()`, computed by masking.
    ///
    /// This is intended for converting a bit address into a bit offset
    /// within a block, which is why it takes `u64` and returns `usize`.
    #[inline]
    fn mod_nbits(index: u64) -> usize {
        let mask: u64 = Self::lg_nbits_mask();
        (index & mask) as usize
    }

    /// Returns `index * Self::nbits()`, computed by shifting.
    ///
    /// This is intended for converting a block address into a bit address,
    /// which is why it takes a `usize` and returns a `u64`.
    fn mul_nbits(index: usize) -> u64 {
        (index as u64) << Self::lg_nbits()
    }

    /// Computes how many bits are in the last block of an array of
    /// `len` bits.
    ///
    /// This is like `Self::mod_nbits`, but it returns `Self::nbits()` in
    /// lieu of 0. Note that this means that if you have 0 bits then the
    /// last block is full.
    #[inline]
    fn last_block_bits(len: u64) -> usize {
        let masked = Self::mod_nbits(len);
        if masked == 0 {
            Self::nbits()
        } else {
            masked
        }
    }

    /// Log-base-2 of the number of bits in a block.
    #[inline]
    fn lg_nbits() -> usize {
        Self::nbits().floor_lg()
    }

    /// Mask with the lowest-order `lg_nbits()` set.
    #[inline]
    fn lg_nbits_mask<Result: BlockType>() -> Result {
        Result::low_mask(Self::lg_nbits())
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
    /// BitVec are index in little-endian style based at 0.
    ///
    /// # Precondition
    ///
    /// `bit_index < Self::nbits()`
    #[inline]
    fn nth_mask(bit_index: usize) -> Self {
        Self::one() << bit_index
    }

    // Methods for getting and setting bits.

    /// Extracts the value of the `bit_index`th bit.
    ///
    /// # Panics
    ///
    /// Panics if `bit_index` is out of bounds.
    #[inline]
    fn get_bit(self, bit_index: usize) -> bool {
        assert!(bit_index < Self::nbits(), "Block::get_bit: out of bounds");
        self & Self::nth_mask(bit_index) != Self::zero()
    }

    /// Functionally updates the value of the `bit_index`th bit to `bit_value`.
    ///
    /// # Panics
    ///
    /// Panics if `bit_index` is out of bounds.
    #[inline]
    fn with_bit(self, bit_index: usize, bit_value: bool) -> Self {
        assert!(bit_index < Self::nbits(), "Block::with_bit: out of bounds");
        if bit_value {
            self | Self::nth_mask(bit_index)
        } else {
            self & !Self::nth_mask(bit_index)
        }
    }

    /// Extracts `len` bits starting at bit offset `start`.
    ///
    /// # Panics
    ///
    /// Panics of the bit span is out of bounds.
    #[inline]
    fn get_bits(self, start: usize, len: usize) -> Self {
        assert!(
            start + len <= Self::nbits(),
            "Block::get_bits: out of bounds"
        );;

        if len == 0 {
            return Self::zero();
        }

        (self >> start) & Self::low_mask(len)
    }

    /// Functionally updates `len` bits to `value` starting at offset `start`.
    ///
    /// # Panics
    ///
    /// Panics of the bit span is out of bounds.
    #[inline]
    fn with_bits(self, start: usize, len: usize, value: Self) -> Self {
        assert!(
            start + len <= Self::nbits(),
            "Block::with_bits: out of bounds"
        );

        if len == 0 {
            return self;
        }

        let mask = Self::low_mask(len) << start;
        let shifted_value = value << start;

        (self & !mask) | (shifted_value & mask)
    }

    // Arithmetic methods that probably belong elsewhere.

    /// Returns the smallest number `n` such that `2.pow(n) >= self`.
    #[inline]
    fn ceil_lg(self) -> usize {
        if self <= Self::one() {
            return 0;
        }
        Self::nbits() - (self - Self::one()).leading_zeros() as usize
    }

    /// Returns the largest number `n` such that `2.pow(n) <= self`.
    #[inline]
    fn floor_lg(self) -> usize {
        if self <= Self::one() {
            return 0;
        }
        Self::nbits() - 1 - self.leading_zeros() as usize
    }

    /// Returns the smallest number `n` such that `n * divisor >= self`.
    #[inline]
    fn ceil_div(self, divisor: Self) -> Self {
        (self + divisor - Self::one()) / divisor
    }

    // I/O methods

    /// Reads a block with the specified endianness.
    fn read_block<R, T>(source: &mut R) -> io::Result<Self>
    where
        R: io::Read,
        T: ByteOrder;

    /// Writes a block with the specified endianness.
    fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
    where
        W: io::Write,
        T: ByteOrder;
}

macro_rules! fn_low_mask {
    ( $ty:ident ) => {
        #[inline]
        fn low_mask(k: usize) -> $ty {
            debug_assert!(k <= Self::nbits());

            // Compute the mask when element_bits is not the word size:
            let a = $ty::one().wrapping_shl(k as u32) - 1;

            // Special case for the word size:
            let b = (Self::div_nbits(k as u64) & 1) as $ty * !0;

            a | b
        }
    };
}

impl BlockType for u8 {
    fn read_block<R, T>(source: &mut R) -> io::Result<Self>
    where
        R: io::Read,
        T: ByteOrder,
    {
        source.read_u8()
    }

    fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
    where
        W: io::Write,
        T: ByteOrder,
    {
        sink.write_u8(*self)
    }

    fn_low_mask!(u8);
}

macro_rules! impl_block_type {
    ($ty:ident, $read:ident, $write:ident) => {
        impl BlockType for $ty {
            fn read_block<R, T>(source: &mut R) -> io::Result<Self>
            where
                R: io::Read,
                T: ByteOrder,
            {
                source.$read::<T>()
            }

            fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
            where
                W: io::Write,
                T: ByteOrder,
            {
                sink.$write::<T>(*self)
            }

            fn_low_mask!($ty);
        }
    };
}

impl_block_type!(u16, read_u16, write_u16);
impl_block_type!(u32, read_u32, write_u32);
impl_block_type!(u64, read_u64, write_u64);

impl BlockType for usize {
    #[cfg(target_pointer_width = "64")]
    fn read_block<R, T>(source: &mut R) -> io::Result<Self>
    where
        R: io::Read,
        T: ByteOrder,
    {
        source.read_u64::<T>().map(|x| x as usize)
    }

    #[cfg(target_pointer_width = "32")]
    fn read_block<R, T>(source: &mut R) -> io::Result<Self>
    where
        R: io::Read,
        T: ByteOrder,
    {
        source.read_u32::<T>().map(|x| x as usize)
    }

    #[cfg(target_pointer_width = "64")]
    fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
    where
        W: io::Write,
        T: ByteOrder,
    {
        sink.write_u64::<T>(*self as u64)
    }

    #[cfg(target_pointer_width = "32")]
    fn write_block<W, T>(&self, sink: &mut W) -> io::Result<()>
    where
        W: io::Write,
        T: ByteOrder,
    {
        sink.write_u32::<T>(*self as u32)
    }

    fn_low_mask!(usize);
}

/// Represents the address of a bit, broken into a block component
/// and a bit offset component.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Address {
    /// The index of the block containing the bit in question.
    pub block_index: usize,
    /// The position of the bit in question within its block.
    pub bit_offset: usize,
}

impl Address {
    /// Creates an `Address` for the given bit index for storage in
    /// block type `Block`.
    ///
    /// # Panics
    ///
    /// Panics if `bit_index` divided by the block size doesn’t fit in a
    /// `usize`.
    #[inline]
    pub fn new<Block: BlockType>(bit_index: u64) -> Self {
        Address {
            block_index: Block::checked_div_nbits(bit_index).expect("Address::new: index overflow"),
            bit_offset: Block::mod_nbits(bit_index),
        }
    }

    /// Converts an `Address` back into a raw bit index.
    ///
    /// This method and `new` should be inverses.
    #[inline]
    pub fn bit_index<Block: BlockType>(&self) -> u64 {
        Block::mul_nbits(self.block_index) + self.bit_offset as u64
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{quickcheck, TestResult};

    #[test]
    fn low_mask() {
        assert_eq!(0b00011111, u8::low_mask(5));
        assert_eq!(0b0011111111111111, u16::low_mask(14));
        assert_eq!(0b1111111111111111, u16::low_mask(16));
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
        assert_eq!(0b0, 0b0100110001110000u16.get_bits(0, 0));
        assert_eq!(0b010, 0b0100110001110000u16.get_bits(13, 3));
        assert_eq!(0b110001, 0b0100110001110000u16.get_bits(6, 6));
        assert_eq!(0b10000, 0b0100110001110000u16.get_bits(0, 5));
        assert_eq!(0b0100110001110000, 0b0100110001110000u16.get_bits(0, 16));
    }

    #[test]
    fn with_bits() {
        assert_eq!(
            0b0111111111000001,
            0b0110001111000001u16.with_bits(10, 3, 0b111)
        );
        assert_eq!(
            0b0101110111000001,
            0b0110001111000001u16.with_bits(9, 5, 0b01110)
        );
        assert_eq!(
            0b0110001111000001,
            0b0110001111000001u16.with_bits(14, 0, 0b01110)
        );
        assert_eq!(
            0b0110001110101010,
            0b0110001111000001u16.with_bits(0, 8, 0b10101010)
        );
        assert_eq!(
            0b0000000000000010,
            0b0110001111000001u16.with_bits(0, 16, 0b10)
        );
    }

    #[test]
    fn get_bit() {
        assert!(!0b00000000u8.get_bit(0));
        assert!(!0b00000000u8.get_bit(1));
        assert!(!0b00000000u8.get_bit(2));
        assert!(!0b00000000u8.get_bit(3));
        assert!(!0b00000000u8.get_bit(7));
        assert!(!0b10101010u8.get_bit(0));
        assert!(0b10101010u8.get_bit(1));
        assert!(!0b10101010u8.get_bit(2));
        assert!(0b10101010u8.get_bit(3));
        assert!(0b10101010u8.get_bit(7));
    }

    #[test]
    fn with_bit() {
        assert_eq!(0b00100000, 0b00000000u8.with_bit(5, true));
        assert_eq!(0b00000000, 0b00000000u8.with_bit(5, false));
        assert_eq!(0b10101010, 0b10101010u8.with_bit(7, true));
        assert_eq!(0b00101010, 0b10101010u8.with_bit(7, false));
        assert_eq!(0b10101011, 0b10101010u8.with_bit(0, true));
        assert_eq!(0b10101010, 0b10101010u8.with_bit(0, false));
    }

    #[test]
    fn floor_lg() {
        assert_eq!(0, 1u32.floor_lg());
        assert_eq!(1, 2u32.floor_lg());
        assert_eq!(1, 3u32.floor_lg());
        assert_eq!(2, 4u32.floor_lg());
        assert_eq!(2, 5u32.floor_lg());
        assert_eq!(2, 7u32.floor_lg());
        assert_eq!(3, 8u32.floor_lg());

        fn prop(n: u64) -> TestResult {
            if n == 0 {
                return TestResult::discard();
            }

            TestResult::from_bool(
                2u64.pow(n.floor_lg() as u32) <= n && 2u64.pow(n.floor_lg() as u32 + 1) > n,
            )
        }

        quickcheck(prop as fn(u64) -> TestResult);
    }

    #[test]
    fn ceil_lg() {
        assert_eq!(0, 1u32.ceil_lg());
        assert_eq!(1, 2u32.ceil_lg());
        assert_eq!(2, 3u32.ceil_lg());
        assert_eq!(2, 4u32.ceil_lg());
        assert_eq!(3, 5u32.ceil_lg());
        assert_eq!(3, 7u32.ceil_lg());
        assert_eq!(3, 8u32.ceil_lg());
        assert_eq!(4, 9u32.ceil_lg());

        fn prop(n: u64) -> TestResult {
            if n <= 1 {
                return TestResult::discard();
            }

            TestResult::from_bool(
                2u64.pow(n.ceil_lg() as u32) >= n && 2u64.pow(n.ceil_lg() as u32 - 1) < n,
            )
        }

        quickcheck(prop as fn(u64) -> TestResult);
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

        fn prop(n: u64, m: u64) -> TestResult {
            if n * m == 0 {
                return TestResult::discard();
            }

            TestResult::from_bool(m * n.ceil_div(m) >= n && m * (n.ceil_div(m) - 1) < n)
        }

        quickcheck(prop as fn(u64, u64) -> TestResult);
    }
}

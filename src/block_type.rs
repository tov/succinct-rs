//! A trait describing types that can be used for `IntVec` storage.

use std::mem;

use num::PrimInt;

/// Types that can be used for `IntVec` storage.
pub trait BlockType: PrimInt {
    /// The number of bits in a block.
    #[inline(always)]
    fn nbits() -> usize {
        8 * mem::size_of::<Self>()
    }

    /// The bit mask consisting of 0s followed by `element_bits` ones.
    ///
    /// # Precondition
    ///
    /// `element_bits <= Self::nbits()`
    #[inline]
    fn element_mask(element_bits: usize) -> Self {
        debug_assert!(element_bits <= Self::nbits());

        if element_bits == Self::nbits() {
            !Self::zero()
        } else {
            (Self::one() << element_bits) - Self::one()
        }
    }

    /// The bit mask with the `bit_index`th bit set.
    ///
    /// # Precondition
    ///
    /// `bit_index` < Self::nbits()`
    #[inline]
    fn nth_mask(bit_index: usize) -> Self {
        debug_assert!(bit_index < Self::nbits());

        Self::one() << (Self::nbits() - bit_index - 1)
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

        (self >> (Self::nbits() - limit)) & Self::element_mask(len)
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

        let after_bits = Self::nbits() - limit;
        let mask = Self::element_mask(len) << after_bits;
        let shifted_value = value << after_bits;

        (self & !mask) | (shifted_value & mask)
    }

    /// Extracts the value of the `bit_index`th bit.
    #[inline]
    fn get_bit(self, bit_index: usize) -> bool {
        debug_assert!(bit_index <= Self::nbits());
        self & Self::nth_mask(bit_index) != Self::zero()
    }

    /// Sets the value of the `bit_index`th bit to true.
    #[inline]
    fn set_bit(self, bit_index: usize, bit_value: bool) -> Self {
        debug_assert!(bit_index <= Self::nbits());
        if bit_value {
            self | Self::nth_mask(bit_index)
        } else {
            self & !Self::nth_mask(bit_index)
        }
    }
}

impl<Block: PrimInt> BlockType for Block { }

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn element_mask() {
        assert_eq!(0b00011111, u8::element_mask(5));
        assert_eq!(0b0011111111111111, u16::element_mask(14));
    }

    #[test]
    fn nth_mask() {
        assert_eq!(0b10000000, u8::nth_mask(0));
        assert_eq!(0b01000000, u8::nth_mask(1));
        assert_eq!(0b00100000, u8::nth_mask(2));
        assert_eq!(0b00000001, u8::nth_mask(7));
    }

    #[test]
    fn get_bits() {
        assert_eq!(0b0,
                   0b0100110001110000u16.get_bits(0, 0));
        assert_eq!(0b010,
                   0b0100110001110000u16.get_bits(0, 3));
        assert_eq!(    0b110001,
                   0b0100110001110000u16.get_bits(4, 6));
        assert_eq!(           0b10000,
                   0b0100110001110000u16.get_bits(11, 5));
        assert_eq!(0b0100110001110000,
                   0b0100110001110000u16.get_bits(0, 16));
    }

    #[test]
    fn set_bits() {
        assert_eq!(0b0111111111000001,
                   0b0110001111000001u16.set_bits(3, 3, 0b111));
        assert_eq!(0b0101110111000001,
                   0b0110001111000001u16.set_bits(2, 5, 0b01110));
        assert_eq!(0b0110001111000001,
                   0b0110001111000001u16.set_bits(2, 0, 0b01110));
        assert_eq!(0b0110001110101010,
                   0b0110001111000001u16.set_bits(8, 8, 0b10101010));
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
        assert!(  0b10101010u8.get_bit(0));
        assert!(! 0b10101010u8.get_bit(1));
        assert!(  0b10101010u8.get_bit(2));
        assert!(! 0b10101010u8.get_bit(3));
        assert!(! 0b10101010u8.get_bit(7));
    }

    #[test]
    fn set_bit() {
        assert_eq!(0b00100000, 0b00000000u8.set_bit(2, true));
        assert_eq!(0b00000000, 0b00000000u8.set_bit(2, false));
        assert_eq!(0b10101010, 0b10101010u8.set_bit(0, true));
        assert_eq!(0b00101010, 0b10101010u8.set_bit(0, false));
        assert_eq!(0b10101011, 0b10101010u8.set_bit(7, true));
        assert_eq!(0b10101010, 0b10101010u8.set_bit(7, false));
    }
}

use std::ops::Range;

use bit_vec::traits::*;
use space_usage::SpaceUsage;

/// A borrowed slice of a bit vector.
#[derive(Clone, Copy, Debug)]
pub struct BitSlice<'a, Base: 'a + BitVec> {
    data: &'a Base,
    start: u64,
    len: u64,
}

/// A borrowed, mutable slice of a bit vector.
#[derive(Debug)]
pub struct BitSliceMut<'a, Base: 'a + BitVecMut> {
    data: &'a mut Base,
    start: u64,
    len: u64,
}

impl<'a, Base: 'a + BitVec> BitSlice<'a, Base> {
    /// Slices base to the specified range.
    pub fn new(base: &'a Base, range: Range<u64>) -> Self {
        assert!(range.end < base.bit_len(), "BitSlice::new: out of bounds");
        BitSlice {
            data: base,
            start: range.start,
            len: range.end - range.start,
        }
    }

    // TODO: slice
}

impl<'a, Base: 'a + BitVecMut> BitSliceMut<'a, Base> {
    /// Slices base to the specified range.
    pub fn new(base: &'a mut Base, range: Range<u64>) -> Self {
        assert!(range.end < base.bit_len(), "BitSlice::new: out of bounds");
        BitSliceMut {
            data: base,
            start: range.start,
            len: range.end - range.start,
        }
    }

    // TODO: slice_mut
}

impl<'a, Base: 'a + BitVec> BitVec for BitSlice<'a, Base> {
    type Block = Base::Block;

    #[inline]
    fn bit_len(&self) -> u64 {
        self.len
    }

    #[inline]
    fn get_bit(&self, position: u64) -> bool {
        assert!(position < self.len, "BitSlice::get_bit: out of bounds");
        self.data.get_bit(self.start + position)
    }

    // TODO: efficient get_block
}

impl<'a, Base: 'a + BitVecMut> BitVec for BitSliceMut<'a, Base> {
    type Block = Base::Block;

    #[inline]
    fn bit_len(&self) -> u64 {
        self.len
    }

    #[inline]
    fn get_bit(&self, position: u64) -> bool {
        assert!(position < self.len, "BitSlice::get_bit: out of bounds");
        self.data.get_bit(self.start + position)
    }

    // TODO: efficient get_block
}

impl<'a, Base: 'a + BitVecMut> BitVecMut for BitSliceMut<'a, Base> {
    #[inline]
    fn set_bit(&mut self, position: u64, value: bool) {
        assert!(position < self.len, "BitSlice::set_bit: out of bounds");
        let start = self.start;
        self.data.set_bit(start + position, value);
    }

    // TODO: efficient set_block
}

impl<'a, Base: 'a + BitVec> SpaceUsage for BitSlice<'a, Base> {
    fn is_stack_only() -> bool { true }
    fn heap_bytes(&self) -> usize { 0 }
}

impl<'a, Base: 'a + BitVecMut> SpaceUsage for BitSliceMut<'a, Base> {
    fn is_stack_only() -> bool { true }
    fn heap_bytes(&self) -> usize { 0 }
}

//! Bit-packed vectors of *k*-bit unsigned integers.

use std::{fmt, mem};

use num::{PrimInt, ToPrimitive};

use storage::{BitStore, BitStoreMut, BlockType};
use space_usage::SpaceUsage;

pub mod builder;
pub use self::builder::IntVecBuilder;

/// A vector of *k*-bit unsigned integers, where *k* is determined at
/// run time.
///
/// Construct with [`IntVec::new`](#method.new), or for more control,
/// [`IntVecBuilder`](struct.IntVecBuilder.html).
///
/// `Block` gives the representation type. The element size *k* can
/// never exceed the number of bits in `Block`.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct IntVec<Block: BlockType = usize> {
    blocks: Vec<Block>,
    n_elements: u64,
    element_bits: usize,
}

/// Describes how to initialize the memory of an `IntVec`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fill<Block: BlockType = usize> {
    /// Initialize each block—not each element—to the value.
    Block(Block),
    /// Initialize each element to the value. (What should happen to
    /// extra bits? Mask out or panic?)
    Element(Block),
}

/// The address of a bit, as an index to a block and the index of a bit
/// in that block.
#[derive(Clone, Copy, Debug)]
struct Address {
    block_index: usize,
    bit_offset: usize,
}

impl Address {
    fn new(bit_index: u64, block_bits: usize) -> Self {
        let block_bits = block_bits as u64;
        Address {
            block_index: (bit_index / block_bits) as usize,
            bit_offset: (bit_index % block_bits) as usize,
        }
    }
}

impl<Block: PrimInt> IntVec<Block> {
    // Computes the number of blocks from the number of elements.
    // Performs sufficient overflow checks that we shouldn’t have to
    // repeat them each time we index, even though it’s nearly the
    // same calculation.
    #[inline]
    fn compute_n_blocks(element_bits: usize, n_elements: u64)
                        -> Result<usize, &'static str> {

        // We perform the size calculation explicitly in u64. This
        // is because we use a bit size, which limits us to 1/8 of a
        // 32-bit address space when usize is 32 bits. Instead, we
        // perform the calculation in 64 bits and check for overflow.
        let element_bits = element_bits as u64;
        let block_bits   = Self::block_bits() as u64;

        if element_bits == 0 {
            Err("IntVec: cannot have 0-bit elements.")
        } else if block_bits < element_bits {
            Err("IntVec: element size cannot exceed block size.")
        } else if let Some(n_bits) = n_elements.checked_mul(element_bits) {
            let mut result = n_bits / block_bits;
            if n_bits % block_bits > 0 { result += 1 }
            result.to_usize().ok_or("IntVec: size overflow (usize)")
        } else {
            Err("IntVec: size overflow (checked_mul)")
        }
    }

    #[inline]
    fn compute_address_random(&self, bit_offset: u64, element_bits: usize,
                              element_index: u64) -> Address {
        let bits_index = element_index
            .checked_mul(element_bits as u64)
            .expect("IntVec: index overflow")
            .checked_add(bit_offset)
            .expect("IntVec: index overflow");

        let bits_limit = bits_index + element_bits as u64;
        assert!(bits_limit <= (Self::block_bits() * self.blocks.len()) as u64,
                "IntVec: index out of bounds.");

        Address::new(bits_index, Self::block_bits())
    }

    #[inline]
    fn compute_address(&self, element_index: u64) -> Address {
        // Because of the underlying slice, this bounds checks twice.
        assert!(element_index < self.n_elements,
                "IntVec: index out of bounds.");

        // As before we perform the index calculation explicitly in
        // u64. The bounds check at the top of this method, combined
        // with the overflow checks at construction time, mean we don’t
        // need to worry about overflows here.
        let element_bits  = self.element_bits() as u64;
        let bit_index     = element_index * element_bits;

        Address::new(bit_index, Self::block_bits())
    }

    /// Creates a new integer vector.
    ///
    /// # Arguments
    ///
    ///  - `element_bits` — the size of each element in bits; hence
    ///    elements range from `0` to `2.pow(element_bits) - 1`.
    ///
    /// # Result
    ///
    /// The new, empty integer vector.
    pub fn new(element_bits: usize) -> Self {
        IntVecBuilder::<Block>::new(element_bits).build()
    }

    /// Determines whether we can support a vector with the given
    /// element size and number of elements.
    ///
    /// We cannot support vectors where:
    ///
    ///   - `block_bits() < element_bits`;
    ///   - `n_elements * element_bits` doesn’t fit in a `u64`; or
    ///   - `n_elements * element_bits / block_bits()` (but with the
    ///     division rounded up) doesn’t fit in a `usize`.
    ///
    /// where `block_bits()` is the size of the `Block` type parameter.
    pub fn is_okay_size(element_bits: usize, n_elements: u64) -> bool {
        Self::compute_n_blocks(element_bits, n_elements).is_ok()
    }

    /// Returns the number of elements in the vector.
    #[inline]
    pub fn len(&self) -> u64 {
        self.n_elements
    }

    /// Is the vector empty?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_address(&self, address: Address, element_bits: usize) -> Block {
        let block_bits = Self::block_bits();
        let margin = block_bits - address.bit_offset;

        if margin >= element_bits {
            let block = self.blocks[address.block_index];
            return block.get_bits(address.bit_offset, element_bits)
        }

        let extra = element_bits - margin;

        let block1 = self.blocks[address.block_index];
        let block2 = self.blocks[address.block_index + 1];

        let high_bits = block1.get_bits(address.bit_offset, margin);
        let low_bits = block2.get_bits(0, extra);

        (high_bits << extra) | low_bits
    }

    /// Returns the element at the given index.
    ///
    /// # Panics
    ///
    /// Panics if `element_index` is out of bounds.
    pub fn get(&self, element_index: u64) -> Block {
        if self.is_packed() {
            return self.blocks[element_index as usize];
        }

        let element_bits = self.element_bits();

        if element_bits == 1 {
            if self.get_bit(element_index) {
                return Block::one();
            } else {
                return Block::zero();
            }
        }

        let address = self.compute_address(element_index);
        self.get_address(address, element_bits)
    }

    /// Returns the element at a given index, also given an arbitrary
    /// element size and bit offset.
    ///
    /// This computes the location of the `element_index`th element
    /// supposing that elements are `element_bits` side, then adds
    /// `bit_offset` additional bits and returns the `element_bits`-bit
    /// value found at that location.
    ///
    /// # Panics
    ///
    /// Panics if the referenced bits are out of bounds. Bounds are
    /// considered to the end of the support array, even if that goes
    /// past the last element of the `IntArray`.
    pub fn get_random(&self,
                      bit_offset: u64,
                      element_bits: usize,
                      element_index: u64) -> Block {
        let address = self.compute_address_random(bit_offset,
                                                  element_bits,
                                                  element_index);
        self.get_address(address, element_bits)
    }

    fn set_address(&mut self, address: Address, element_bits: usize,
                   element_value: Block) {
        let block_bits = Self::block_bits();
        let margin = block_bits - address.bit_offset;

        if margin >= element_bits {
            let old_block = self.blocks[address.block_index];
            let new_block = old_block.set_bits(address.bit_offset,
                                               element_bits,
                                               element_value);
            self.blocks[address.block_index] = new_block;
            return;
        }

        let extra = element_bits - margin;

        let old_block1 = self.blocks[address.block_index];
        let old_block2 = self.blocks[address.block_index + 1];

        let high_bits = element_value >> extra;

        let new_block1 = old_block1.set_bits(address.bit_offset,
                                             margin, high_bits);
        let new_block2 = old_block2.set_bits(0, extra, element_value);

        self.blocks[address.block_index] = new_block1;
        self.blocks[address.block_index + 1] = new_block2;
    }

    /// Sets the element at the given index.
    ///
    /// # Panics
    ///
    ///   - Panics if `element_index` is out of bounds.
    ///
    ///   - Debug mode only: Panics if `element_value` is too large to
    ///     fit in the element size. (TODO: What’s the right thing here?)
    pub fn set(&mut self, element_index: u64, element_value: Block) {
        if self.is_packed() {
            self.blocks[element_index as usize] = element_value;
            return;
        }

        let element_bits = self.element_bits();

        debug_assert!(element_value < Block::one() << element_bits,
                      "IntVec::set: value overflow");

        if element_bits == 1 {
            self.set_bit(element_index, element_value == Block::one());
            return;
        }

        let address = self.compute_address(element_index);
        self.set_address(address, element_bits, element_value);
    }


    /// Sets the element at a given index to a given value, also given
    /// an arbitrary element size and bit offset.
    ///
    /// This computes the location of the `element_index`th element
    /// supposing that elements are `element_bits` side, then adds
    /// `bit_offset` additional bits and modifies the `element_bits`-bit
    /// value found at that location.
    ///
    /// # Panics
    ///
    ///   - Panics if the referenced bits are out of bounds. Bounds are
    ///     considered to the end of the support array, even if that goes
    ///     past the last element of the `IntArray`.
    ///
    ///   - Debug mode only: Panics if `element_value` is too large to
    ///     fit in the element size. (TODO: What’s the right thing here?)
    pub fn set_random(&mut self, bit_offset: u64, element_bits: usize,
                      element_index: u64, element_value: Block) {
        debug_assert!(element_value < Block::one() << element_bits,
                      "IntVec::set_random: value overflow");

        let address = self.compute_address_random(bit_offset,
                                                  element_bits,
                                                  element_index);
        self.set_address(address, element_bits, element_value);
    }

    /// Pushes an element onto the end of the vector, increasing the
    /// length by 1.
    pub fn push(&mut self, element_value: Block) {
        if self.n_elements >= self.backing_len() {
            self.blocks.push(Block::zero());
        }

        let pos = self.n_elements;
        self.n_elements = pos + 1;
        self.set(pos, element_value);
    }

    /// Removes and returns the last element of the vector, if present.
    pub fn pop(&mut self) -> Option<Block> {
        if self.n_elements == 0 {
            None
        } else {
            let result = self.get(self.n_elements - 1);
            self.n_elements -= 1;
            Some(result)
        }
    }

    /// The number of elements the vector can hold without reallocating.
    pub fn capacity(&self) -> u64 {
        let total_bits = self.blocks.capacity() as u64 * Block::nbits() as u64;
        total_bits / self.element_bits as u64
    }

    /// How many elements the backing vector has expanded to store.
    fn backing_len(&self) -> u64 {
        let total_bits = self.blocks.len() as u64 * Block::nbits() as u64;
        total_bits / self.element_bits as u64
    }

    /// Resizes to the given number of elements, filling if necessary.
    pub fn resize(&mut self, n_elements: u64, fill: Fill<Block>) {
        if n_elements <= self.n_elements {
            self.n_elements = n_elements;
        } else {
            match fill {
                Fill::Block(block) => {
                    let n_blocks = Self::compute_n_blocks(self.element_bits,
                                                          n_elements)
                        .unwrap();
                    self.blocks.resize(n_blocks, block);
                    self.n_elements = n_elements;
                }

                Fill::Element(element) => {
                    for _ in self.n_elements .. n_elements {
                        self.push(element);
                    }
                }
            }
        }
    }

    /// Reserves capacity for at least `additional` more elements to be
    /// inserted in the given `IntVec<Block>`.
    ///
    /// The collection may reserve more space to avoid frequent
    /// reallocations.
    ///
    /// # Panics
    ///
    /// Panics if the size conditions of
    /// [`IntVec::<Block>::is_okay_size()`](struct.IntVec.html#method.is_okay_size)
    /// are not met. This will happen if the total number of bits
    /// overflows `u64`.
    pub fn reserve(&mut self, additional: u64) {
        let goal_elements = self.len().checked_add(additional)
            .expect("IntVec::reserve: size overflow");
        let goal_blocks = Self::compute_n_blocks(self.element_bits,
                                                 goal_elements)
            .unwrap();
        let difference = self.blocks.capacity().saturating_sub(goal_blocks);
        self.blocks.reserve(difference);
    }

    /// Reserves capacity for at least `additional` more elements to be
    /// inserted in the given `IntVec<Block>`.
    ///
    /// Unlike [`reserve`](#method.reserve), does nothing if the
    /// capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the size conditions of
    /// [`IntVec::<Block>::is_okay_size()`](struct.IntVec.html#method.is_okay_size)
    /// are not met. This will happen if the total number of bits
    /// overflows `u64`.
    pub fn reserve_exact(&mut self, additional: u64) {
        let goal_elements = self.len().checked_add(additional)
            .expect("IntVec::reserve: size overflow");
        let goal_blocks = Self::compute_n_blocks(self.element_bits,
                                                 goal_elements)
            .unwrap();
        let difference = self.blocks.capacity().saturating_sub(goal_blocks);
        self.blocks.reserve_exact(difference);
    }

    /// Shrinks the capacity to just fit the number of elements.
    pub fn shrink_to_fit(&mut self) {
        let n_blocks = Self::compute_n_blocks(self.element_bits,
                                              self.n_elements).unwrap();
        self.blocks.truncate(n_blocks);
        self.blocks.shrink_to_fit();
    }

    /// Shrinks to the given size.
    ///
    /// If `n_elements` is greater than the current size, does nothing.
    pub fn truncate(&mut self, n_elements: u64) {
        if n_elements <= self.n_elements {
            self.n_elements = n_elements;
        }
    }

    /// Returns a reference to the backing slice of blocks.
    ///
    /// Note that this does not respect element boundaries, and the
    /// layout is not specified.
    pub fn as_block_slice(&self) -> &[Block] {
        &self.blocks
    }

    /// Returns a mutable reference to the backing slice of blocks.
    ///
    /// Note that this does not respect element boundaries, and the
    /// layout is not specified.
    pub fn as_mut_block_slice(&mut self) -> &mut [Block] {
        &mut self.blocks
    }

    /// Sets the size to 0 while retaining the allocated storage.
    pub fn clear(&mut self) {
        self.n_elements = 0;
    }

    /// Gets an iterator over the elements of the vector.
    pub fn iter(&self) -> Iter<Block> {
        Iter {
            vec: self,
            start: 0,
            limit: self.len()
        }
    }

    /// The number of bits per block of storage.
    #[inline]
    pub fn block_bits() -> usize {
        Block::nbits()
    }

    /// The number of bits per elements.
    #[inline]
    pub fn element_bits(&self) -> usize {
        self.element_bits
    }

    /// True if elements are packed one per block.
    #[inline]
    pub fn is_packed(&self) -> bool {
        self.element_bits() == Self::block_bits()
    }

    /// True if elements are aligned within blocks.
    #[inline]
    pub fn is_aligned(&self) -> bool {
        Self::block_bits() % self.element_bits() == 0
    }

    // TODO: fn align(&mut self) chooses element_bits...

}

impl<Block: BlockType> BitStore for IntVec<Block> {
    type Block = Block;

    fn block_len(&self) -> usize {
        self.blocks.len()
    }

    fn bit_len(&self) -> u64 {
        (self.element_bits as u64) * (self.n_elements as u64)
    }

    fn get_block(&self, position: usize) -> Block {
        self.blocks[position]
    }
}

impl<Block: BlockType> BitStoreMut for IntVec<Block> {
    fn set_block(&mut self, position: usize, value: Block) {
        self.blocks[position] = value;
    }
}

/// An iterator over the elements of an [`IntVec`](struct.IntVec.html).
pub struct Iter<'a, Block: BlockType + 'a = usize> {
    vec: &'a IntVec<Block>,
    start: u64,
    limit: u64,
}

impl<'a, Block: BlockType> Iterator for Iter<'a, Block> {
    type Item = Block;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.limit {
            let result = self.vec.get(self.start);
            self.start += 1;
            Some(result)
        } else { None }
    }

    #[cfg(target_pointer_width = "32")]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(len) = (self.limit - self.start).to_usize() {
            (len, Some(len))
        } else {
            (0, None)
        }
    }

    #[cfg(target_pointer_width = "64")]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }

    fn count(self) -> usize {
        self.len()
    }

    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.start = self.start.checked_add(n as u64).unwrap_or(self.limit);
        self.next()
    }
}

#[cfg(target_pointer_width = "64")]
impl<'a, Block: BlockType> ExactSizeIterator for Iter<'a, Block> {
    fn len(&self) -> usize {
        (self.limit - self.start) as usize
    }
}

impl<'a, Block: BlockType> DoubleEndedIterator for Iter<'a, Block> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start < self.limit {
            self.limit -= 1;
            Some(self.vec.get(self.limit))
        } else { None }

    }
}

impl<'a, Block: BlockType + 'a> IntoIterator for &'a IntVec<Block> {
    type Item = Block;
    type IntoIter = Iter<'a, Block>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<Block> fmt::Debug for IntVec<Block>
        where Block: BlockType + fmt::Debug {

    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(formatter, "IntVec {{ element_bits: {}, elements: {{ ",
                    self.element_bits()));

        for element in self {
            try!(write!(formatter, "{:?}, ", element));
        }

        write!(formatter, "}} }}")
    }
}

impl<A: BlockType> SpaceUsage for IntVec<A> {
    #[inline]
    fn is_stack_only() -> bool { false }

    fn heap_bytes(&self) -> usize {
        self.blocks.capacity() * mem::size_of::<A>()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn create_empty() {
        let v: IntVec = IntVecBuilder::new(4).build();
        assert!(v.is_empty());
    }

    #[test]
    fn packed() {
        let mut v = IntVecBuilder::<u32>::new(32).n_elements(10).build();
        assert_eq!(10, v.len());

        assert_eq!(0, v.get(0));
        assert_eq!(0, v.get(9));

        v.set(0, 89);
        assert_eq!(89, v.get(0));
        assert_eq!(0, v.get(1));

        v.set(0, 56);
        v.set(1, 34);
        assert_eq!(56, v.get(0));
        assert_eq!(34, v.get(1));
        assert_eq!(0, v.get(2));

        v.set(9, 12);
        assert_eq!(12, v.get(9));
    }

    #[test]
    #[should_panic]
    fn packed_oob() {
        let v = IntVecBuilder::<u32>::new(32).n_elements(10).build();
        assert_eq!(0, v.get(10));
    }

    #[test]
    fn aligned() {
        let mut v = IntVecBuilder::new(4).n_elements(20).build();
        assert_eq!(20, v.len());

        assert_eq!(0, v.get(0));
        assert_eq!(0, v.get(9));

        v.set(0, 13);
        assert_eq!(13, v.get(0));
        assert_eq!(0, v.get(1));

        v.set(1, 15);
        assert_eq!(13, v.get(0));
        assert_eq!(15, v.get(1));
        assert_eq!(0, v.get(2));

        v.set(1, 4);
        v.set(19, 9);
        assert_eq!(13, v.get(0));
        assert_eq!(4, v.get(1));
        assert_eq!(0, v.get(2));
        assert_eq!(9, v.get(19));
    }

    #[test]
    #[should_panic]
    fn aligned_oob() {
        let v = IntVecBuilder::new(4).n_elements(20).build();
        assert_eq!(0, v.get(20));
    }

    #[test]
    fn unaligned() {
        let mut v = IntVecBuilder::new(5).n_elements(20).build();
        assert_eq!(20, v.len());

        assert_eq!(0, v.get(0));
        assert_eq!(0, v.get(9));

        v.set(0, 13);
        assert_eq!(13, v.get(0));
        assert_eq!(0, v.get(1));

        v.set(1, 15);
        assert_eq!(13, v.get(0));
        assert_eq!(15, v.get(1));
        assert_eq!(0, v.get(2));

        v.set(1, 4);
        v.set(19, 9);
        assert_eq!(13, v.get(0));
        assert_eq!(4, v.get(1));
        assert_eq!(0, v.get(2));
        assert_eq!(9, v.get(19));
    }

    #[test]
    #[should_panic]
    fn unaligned_oob() {
        let v = IntVecBuilder::new(5).n_elements(20).build();
        assert_eq!(0, v.get(20));
    }

    #[test]
    fn pop() {
        let mut v = IntVec::new(7);
        assert_eq!(None, v.pop());
        v.push(1);
        v.push(2);
        v.push(3);
        assert_eq!(Some(3), v.pop());
        v.push(4);
        v.push(5);
        assert_eq!(Some(5), v.pop());
        assert_eq!(Some(4), v.pop());
        assert_eq!(Some(2), v.pop());
        assert_eq!(Some(1), v.pop());
        assert_eq!(None, v.pop());
    }

    #[test]
    fn iter() {
        let mut v = IntVecBuilder::<u16>::new(13).build();
        v.push(1);
        v.push(1);
        v.push(2);
        v.push(3);
        v.push(5);

        assert_eq!(vec![1, 1, 2, 3, 5], v.iter().collect::<Vec<_>>());
    }

    #[test]
    fn debug() {
        let mut v = IntVecBuilder::<u16>::new(13).build();
        v.push(1);
        v.push(1);
        v.push(2);
        v.push(3);
        v.push(5);

        assert_eq!("IntVec { element_bits: 13, elements: { 1, 1, 2, 3, 5, } }".to_owned(),
                   format!("{:?}", v));
    }

    #[test]
    #[should_panic]
    fn value_overflow() {
        let mut v = IntVec::new(3);
        v.push(78); // 78 is too big
    }

    #[test]
    fn bit_vector() {
        use storage::*;

        let mut v = IntVec::new(1);
        v.push(1);
        v.push(0);
        v.push(0);
        v.push(1);

        assert!(  v.get_bit(0));
        assert!(! v.get_bit(1));
        assert!(! v.get_bit(2));
        assert!(  v.get_bit(3));

        v.set_bit(1, true);

        assert!(  v.get_bit(0));
        assert!(  v.get_bit(1));
        assert!(! v.get_bit(2));
        assert!(  v.get_bit(3));
    }
}

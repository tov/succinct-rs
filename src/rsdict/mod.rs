//! 'RsDict' data structure that supports both rank and select over a bitmap.
//!
//! This crate is an implementation of [Navarro and Providel, "Fast, Small,
//! Simple Rank/Select On
//! Bitmaps"](https://users.dcc.uchile.cl/~gnavarro/ps/sea12.1.pdf), with heavy
//! inspiration from a [Go implementation](https://github.com/hillbig/rsdic).
//!
//! ```
//! use succinct::rsdict::RsDict;
//! use succinct::rank::RankSupport;
//! use succinct::select::SelectSupport;
//!
//! let mut r = RsDict::new();
//! r.push(false);
//! r.push(true);
//! r.push(true);
//! r.push(false);
//!
//! // There's one bit set to the left of index 2.
//! assert_eq!(r.rank(2, true), 1);
//!
//! // The index of the second (zero-indexed as 1) bit is 3.
//! assert_eq!(r.select(1, false), Some(3));
//! ```
//!
//! # Implementation notes
//! First, we store the bitmap in compressed form.  Each block of 64 bits is
//! stored with a variable length code, where the length is determined by the
//! number of bits set in the block (its "class").  Then, we store the classes
//! (i.e. the number of bits set per block) in a separate array, allowing us to
//! iterate forward from a pointer into the variable length buffer.
//!
//! To allow efficient indexing, we then break up the input into
//! `LARGE_BLOCK_SIZE` blocks and store a pointer into the variable length
//! buffer per block.  As with other rank structures, we also store a
//! precomputed rank from the beginning of the large block.
//!
//! Finally, we store precomputed indices for selection in separate arrays.  For
//! every `SELECT_BLOCK_SIZE`th bit, we maintain a pointer to the large block
//! this bit falls in.  We also do the same for zeros.
//!
//! Then, we can compute ranks by consulting the large block rank and then
//! iterating over the small block classes before our desired position.  Once
//! we've found the boundary small block, we can then decode it and compute the
//! rank within the block.  The choice of variable length code allows computing
//! its internal rank without decoding the entire block.
//!
//! Select works similarly where we start with the large block indices, skip
//! over as many small blocks as possible, and then select within a small
//! block. As with rank, we're able to select within a small block directly.

use std::mem;

mod constants;
mod enum_code;

mod rank_acceleration;

#[cfg(test)]
mod test_helpers;

use crate::rank::{BitRankSupport, RankSupport};
use crate::select::{Select0Support, Select1Support, SelectSupport};
use crate::SpaceUsage;

use self::constants::{
    LARGE_BLOCK_SIZE, SELECT_BLOCK_SIZE, SMALL_BLOCK_PER_LARGE_BLOCK, SMALL_BLOCK_SIZE,
};
use self::enum_code::ENUM_CODE_LENGTH;

/// Data structure for efficiently computing both rank and select queries
#[derive(Debug)]
pub struct RsDict {
    len: u64,
    num_ones: u64,
    num_zeros: u64,

    // Small block metadata (stored every SMALL_BLOCK_SIZE bits):
    // * number of set bits (the "class") for the small block
    // * index within a class for each small block; note that the indexes are
    //   variable length (see `ENUM_CODE_LENGTH`), so there isn't direct access
    //   for a particular small block.
    sb_classes: Vec<u8>,
    sb_indices: VarintBuffer,

    // Large block metadata (stored every LARGE_BLOCK_SIZE bits):
    // * pointer into variable-length `bits` for the block start
    // * cached rank at the block start
    large_blocks: Vec<LargeBlock>,

    // Select acceleration:
    // `select_{one,zero}_inds` store the (offset / LARGE_BLOCK_SIZE) of each
    // SELECT_BLOCK_SIZE'th bit.
    select_one_inds: Vec<u64>,
    select_zero_inds: Vec<u64>,

    // Current in-progress small block we're appending to
    last_block: LastBlock,
}

impl RankSupport for RsDict {
    type Over = bool;

    fn rank(&self, pos: u64, bit: bool) -> u64 {
        if pos >= self.len {
            return rank_by_bit(self.num_ones, self.len, bit);
        }
        // If we're in the last block, count the number of ones set after our
        // bit in the last block and remove that from the global count.
        if self.is_last_block(pos) {
            let trailing_ones = self.last_block.count_suffix(pos % SMALL_BLOCK_SIZE);
            return rank_by_bit(self.num_ones - trailing_ones, pos, bit);
        }

        // Start with the rank from our position's large block.
        let lblock = pos / LARGE_BLOCK_SIZE;
        let LargeBlock {
            mut pointer,
            mut rank,
        } = self.large_blocks[lblock as usize];

        // Add in the ranks (i.e. the classes) per small block up to our
        // position's small block.
        let sblock_start = (lblock * SMALL_BLOCK_PER_LARGE_BLOCK) as usize;
        let sblock = (pos / SMALL_BLOCK_SIZE) as usize;
        let (class_sum, length_sum) =
            rank_acceleration::scan_block(&self.sb_classes, sblock_start, sblock);
        rank += class_sum;
        pointer += length_sum;

        // If we aren't on a small block boundary, add in the rank within the small block.
        if pos % SMALL_BLOCK_SIZE != 0 {
            let sb_class = self.sb_classes[sblock];
            let code = self.read_sb_index(pointer, ENUM_CODE_LENGTH[sb_class as usize]);
            rank += enum_code::rank(code, sb_class, pos % SMALL_BLOCK_SIZE);
        }

        rank_by_bit(rank, pos, bit)
    }

    fn limit(&self) -> u64 {
        self.len
    }
}

impl BitRankSupport for RsDict {
    fn rank1(&self, pos: u64) -> u64 {
        self.rank(pos, true)
    }

    fn rank0(&self, pos: u64) -> u64 {
        self.rank(pos, false)
    }
}

impl SelectSupport for RsDict {
    type Over = bool;

    fn select(&self, rank: u64, bit: bool) -> Option<u64> {
        if bit {
            self.select1(rank)
        } else {
            self.select0(rank)
        }
    }
}

impl Select0Support for RsDict {
    fn select0(&self, rank: u64) -> Option<u64> {
        if rank >= self.num_zeros {
            return None;
        }
        // How many zeros are there *excluding* the last block?
        let prefix_num_zeros = self.num_zeros - self.last_block.num_zeros;

        // Our rank must be in the last block.
        if rank >= prefix_num_zeros {
            let lb_rank = (rank - prefix_num_zeros) as u8;
            return Some(self.last_block_ind() + self.last_block.select0(lb_rank));
        }

        // First, use the select pointer to jump forward to a large block and
        // then walk forward over the large blocks until we pass our rank.
        let select_ind = (rank / SELECT_BLOCK_SIZE) as usize;
        let lb_start = self.select_zero_inds[select_ind] as usize;
        let mut lblock = None;
        for (i, large_block) in self.large_blocks[lb_start..].iter().enumerate() {
            let lb_ix = (lb_start + i) as u64;
            let lb_rank = lb_ix * LARGE_BLOCK_SIZE - large_block.rank;
            if rank < lb_rank {
                lblock = Some(lb_ix - 1);
                break;
            }
        }
        let lblock = lblock.unwrap_or(self.large_blocks.len() as u64 - 1);
        let large_block = &self.large_blocks[lblock as usize];

        // Next, iterate over the small blocks, using their cached class to
        // subtract out our rank.
        let sb_start = (lblock * SMALL_BLOCK_PER_LARGE_BLOCK) as usize;
        let mut pointer = large_block.pointer;
        let mut remaining = rank - (lblock * LARGE_BLOCK_SIZE - large_block.rank);
        for (i, &sb_class) in self.sb_classes[sb_start..].iter().enumerate() {
            let sb_zeros = (SMALL_BLOCK_SIZE as u8 - sb_class) as u64;
            let code_length = ENUM_CODE_LENGTH[sb_class as usize];

            // Our desired rank is within this block.
            if remaining < sb_zeros {
                let code = self.read_sb_index(pointer, code_length);
                let sb_rank = (sb_start + i) as u64 * SMALL_BLOCK_SIZE;
                let block_rank = enum_code::select0(code, sb_class, remaining);
                return Some(sb_rank + block_rank);
            }

            // Otherwise, subtract out this block and continue.
            remaining -= sb_zeros;
            pointer += code_length as u64;
        }
        panic!("Ran out of small blocks when iterating over rank");
    }
}

impl Select1Support for RsDict {
    fn select1(&self, rank: u64) -> Option<u64> {
        if rank >= self.num_ones {
            return None;
        }

        let prefix_num_ones = self.num_ones - self.last_block.num_ones;
        if rank >= prefix_num_ones {
            let lb_rank = (rank - prefix_num_ones) as u8;
            return Some(self.last_block_ind() + self.last_block.select1(lb_rank));
        }

        let select_ind = (rank / SELECT_BLOCK_SIZE) as usize;
        let lb_start = self.select_one_inds[select_ind] as usize;
        let mut lblock = None;
        for (i, large_block) in self.large_blocks[lb_start..].iter().enumerate() {
            if rank < large_block.rank {
                lblock = Some((lb_start + i - 1) as u64);
                break;
            }
        }
        let lblock = lblock.unwrap_or(self.large_blocks.len() as u64 - 1);
        let large_block = &self.large_blocks[lblock as usize];

        let sb_start = (lblock * SMALL_BLOCK_PER_LARGE_BLOCK) as usize;
        let mut pointer = large_block.pointer;
        let mut remaining = rank - large_block.rank;
        for (i, &sb_class) in self.sb_classes[sb_start..].iter().enumerate() {
            let sb_ones = sb_class as u64;
            let code_length = ENUM_CODE_LENGTH[sb_class as usize];

            if remaining < sb_ones {
                let code = self.read_sb_index(pointer, code_length);
                let sb_rank = (sb_start + i) as u64 * SMALL_BLOCK_SIZE;
                let block_rank = enum_code::select1(code, sb_class, remaining);
                return Some(sb_rank + block_rank);
            }

            remaining -= sb_ones;
            pointer += code_length as u64;
        }
        panic!("Ran out of small blocks when iterating over rank");
    }
}

impl RsDict {
    /// Create a dictionary from a bitset, specified as an iterator of 64-bit
    /// blocks.  This function is equivalent to pushing each bit one at a time but
    /// is much faster.
    pub fn from_blocks(blocks: impl Iterator<Item = u64>) -> Self {
        if is_x86_feature_detected!("popcnt") {
            unsafe { Self::from_blocks_popcount(blocks) }
        } else {
            Self::from_blocks_impl(blocks)
        }
    }

    #[target_feature(enable = "popcnt")]
    unsafe fn from_blocks_popcount(blocks: impl Iterator<Item = u64>) -> Self {
        Self::from_blocks_impl(blocks)
    }

    #[inline(always)]
    fn from_blocks_impl(blocks: impl Iterator<Item = u64>) -> Self {
        let (_, hint) = blocks.size_hint();
        let hint = hint.unwrap_or(0);

        let mut large_blocks = Vec::with_capacity(hint / LARGE_BLOCK_SIZE as usize);
        let mut select_one_inds = Vec::with_capacity(hint / SELECT_BLOCK_SIZE as usize);
        let mut select_zero_inds = Vec::with_capacity(hint / SELECT_BLOCK_SIZE as usize);
        let mut sb_classes = Vec::with_capacity(hint / SMALL_BLOCK_SIZE as usize);
        let mut sb_indices = VarintBuffer::with_capacity(hint);
        let mut last_block = LastBlock::new();

        let mut num_ones = 0;
        let mut num_zeros = 0;

        let mut iter = blocks.enumerate().peekable();

        while let Some((i, block)) = iter.next() {
            let sb_class = block.count_ones() as u8;

            if i as u64 % SMALL_BLOCK_PER_LARGE_BLOCK == 0 {
                let lblock = LargeBlock {
                    rank: num_ones,
                    pointer: sb_indices.len() as u64,
                };
                large_blocks.push(lblock);
            }

            // If we're on the last block, write to `last_block` rather than
            // pushing onto the `VarintBuffer`.
            if iter.peek().is_none() {
                last_block.bits = block;
                last_block.num_ones = sb_class as u64;
                last_block.num_zeros = 64 - sb_class as u64;
            } else {
                sb_classes.push(sb_class);
                let (code_len, code) = enum_code::encode(block, sb_class);
                sb_indices.push(code_len as usize, code);
            }

            let lb_start = i as u64 * SMALL_BLOCK_SIZE / LARGE_BLOCK_SIZE;

            // We want to see if there's any j in [num_ones, num_ones + sb_class) such
            // that j % SELECT_BLOCK_SIZE = 0.  We can do this arithmetically by
            // comparing two divisors:
            //
            // 1. (num_ones - 1) / SELECT_BLOCK_SIZE and
            // 2. (num_ones + sb_class - 1) / SELECT_BLOCK_SIZE.
            //
            // If they're not equal, there must be a multiple of SELECT_BLOCK_SIZE in
            // the interval [num_ones, num_ones + sb_class).  To see why, consider
            // the case where sb_class > 0 and SELECT_BLOCK_SIZE divides num_ones.
            // Then, the first divisor's numerator is one less than a multiple, and
            // the second one must be greater than or equal to it.  Similarly, if the
            // last value num_ones + sb_class - 1 is a multiple, then the first divsior
            // must be less than the second.  Then, since sb_class < SELECT_BLOCK_SIZE,
            // the same argument holds for any divisor in the middle.
            //
            // Finally, since we're working with unsigned integers, add SELECT_BLOCK_SIZE
            // to both numerators so we don't ever underflow when subtracting one.
            let start = num_ones + SELECT_BLOCK_SIZE - 1;
            let end = num_ones + SELECT_BLOCK_SIZE + sb_class as u64 - 1;
            if start / SELECT_BLOCK_SIZE != end / SELECT_BLOCK_SIZE {
                select_one_inds.push(lb_start);
            }

            // Now do the same for the zero indices.
            let start = num_zeros + SELECT_BLOCK_SIZE - 1;
            let end = num_zeros + SELECT_BLOCK_SIZE + (64 - sb_class as u64) - 1;
            if start / SELECT_BLOCK_SIZE != end / SELECT_BLOCK_SIZE {
                select_zero_inds.push(lb_start);
            }

            num_ones += sb_class as u64;
            num_zeros += 64 - sb_class as u64;
        }

        let num_sb = sb_classes.len();
        let align = SMALL_BLOCK_PER_LARGE_BLOCK as usize;
        sb_classes.reserve((num_sb + align - 1) / align * align);

        Self {
            large_blocks,
            select_one_inds,
            select_zero_inds,
            sb_classes,
            sb_indices,

            len: num_ones + num_zeros,
            num_ones,
            num_zeros,

            last_block,
        }
    }

    /// Create a new `RsDict` with zero capacity.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Create a new `RsDict` with the given capacity preallocated.
    pub fn with_capacity(n: usize) -> Self {
        Self {
            large_blocks: Vec::with_capacity(n / LARGE_BLOCK_SIZE as usize),
            select_one_inds: Vec::with_capacity(n / SELECT_BLOCK_SIZE as usize),
            select_zero_inds: Vec::with_capacity(n / SELECT_BLOCK_SIZE as usize),
            sb_classes: Vec::with_capacity(n / SMALL_BLOCK_SIZE as usize),
            sb_indices: VarintBuffer::with_capacity(n),

            len: 0,
            num_ones: 0,
            num_zeros: 0,

            last_block: LastBlock::new(),
        }
    }

    /// Return the length of the underlying bitmap.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Return whether the underlying bitmap is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Count the number of set bits in the underlying bitmap.
    pub fn count_ones(&self) -> usize {
        self.num_ones as usize
    }

    /// Count the number of unset bits in the underlying bitmap.
    pub fn count_zeros(&self) -> usize {
        self.num_zeros as usize
    }

    /// Push a bit at the end of the underlying bitmap.
    pub fn push(&mut self, bit: bool) {
        if self.len % SMALL_BLOCK_SIZE == 0 {
            self.write_block();
        }
        if bit {
            self.last_block.set_one(self.len % SMALL_BLOCK_SIZE);
            if self.num_ones % SELECT_BLOCK_SIZE == 0 {
                self.select_one_inds.push(self.len / LARGE_BLOCK_SIZE);
            }
            self.num_ones += 1;
        } else {
            self.last_block.set_zero(self.len % SMALL_BLOCK_SIZE);
            if self.num_zeros % SELECT_BLOCK_SIZE == 0 {
                self.select_zero_inds.push(self.len / LARGE_BLOCK_SIZE);
            }
            self.num_zeros += 1;
        }
        self.len += 1;
    }

    /// Query the `pos`th bit (zero-indexed) of the underlying bitmap.
    pub fn get_bit(&self, pos: u64) -> bool {
        if self.is_last_block(pos) {
            return self.last_block.get_bit(pos % SMALL_BLOCK_SIZE);
        }
        let lblock = pos / LARGE_BLOCK_SIZE;
        let sblock = (pos / SMALL_BLOCK_SIZE) as usize;
        let sblock_start = (lblock * SMALL_BLOCK_PER_LARGE_BLOCK) as usize;
        let mut pointer = self.large_blocks[lblock as usize].pointer;
        for &sb_class in &self.sb_classes[sblock_start..sblock] {
            pointer += ENUM_CODE_LENGTH[sb_class as usize] as u64;
        }
        let sb_class = self.sb_classes[sblock];
        let code_length = ENUM_CODE_LENGTH[sb_class as usize];
        let code = self.read_sb_index(pointer, code_length);
        enum_code::decode_bit(code, sb_class, pos % SMALL_BLOCK_SIZE)
    }

    /// Query the `pos`th bit (zero-indexed) of the underlying bit and the
    /// number of set bits to the left of `pos` in a single operation.  This
    /// method is faster than calling `get_bit(pos)` and `rank(pos, true)`
    /// separately.
    pub fn bit_and_one_rank(&self, pos: u64) -> (bool, u64) {
        if self.is_last_block(pos) {
            let sb_pos = pos % SMALL_BLOCK_SIZE;
            let bit = self.last_block.get_bit(sb_pos);
            let after_rank = self.last_block.count_suffix(sb_pos);
            return (bit, self.num_ones - after_rank);
        }
        let lblock = pos / LARGE_BLOCK_SIZE;
        let sblock = (pos / SMALL_BLOCK_SIZE) as usize;
        let sblock_start = (lblock * SMALL_BLOCK_PER_LARGE_BLOCK) as usize;
        let LargeBlock {
            mut pointer,
            mut rank,
        } = self.large_blocks[lblock as usize];
        for &sb_class in &self.sb_classes[sblock_start..sblock] {
            pointer += ENUM_CODE_LENGTH[sb_class as usize] as u64;
            rank += sb_class as u64;
        }
        let sb_class = self.sb_classes[sblock];
        let code_length = ENUM_CODE_LENGTH[sb_class as usize];
        let code = self.read_sb_index(pointer, code_length);

        rank += enum_code::rank(code, sb_class, pos % SMALL_BLOCK_SIZE);
        let bit = enum_code::decode_bit(code, sb_class, pos % SMALL_BLOCK_SIZE);
        (bit, rank)
    }
}

impl RsDict {
    fn write_block(&mut self) {
        if self.len > 0 {
            let block = mem::replace(&mut self.last_block, LastBlock::new());

            let sb_class = block.num_ones as u8;
            self.sb_classes.push(sb_class);

            // To avoid indexing past the end of our allocation when
            // scanning through a large block, reserve some extra space to
            // ensure that we always have a full large block in
            // `sb_classes`.
            let num_sb = self.sb_classes.len();
            let align = SMALL_BLOCK_PER_LARGE_BLOCK as usize;
            self.sb_classes
                .reserve((num_sb + align - 1) / align * align);

            let (code_len, code) = enum_code::encode(block.bits, sb_class);
            self.sb_indices.push(code_len as usize, code);
        }
        if self.len % LARGE_BLOCK_SIZE == 0 {
            let lblock = LargeBlock {
                rank: self.num_ones,
                pointer: self.sb_indices.len() as u64,
            };
            self.large_blocks.push(lblock);
        }
    }

    fn last_block_ind(&self) -> u64 {
        if self.len == 0 {
            return 0;
        }
        ((self.len - 1) / SMALL_BLOCK_SIZE) * SMALL_BLOCK_SIZE
    }

    fn is_last_block(&self, pos: u64) -> bool {
        pos >= self.last_block_ind()
    }

    fn read_sb_index(&self, ptr: u64, code_len: u8) -> u64 {
        self.sb_indices.get(ptr as usize, code_len as usize)
    }
}

impl SpaceUsage for RsDict {
    fn is_stack_only() -> bool {
        false
    }

    fn heap_bytes(&self) -> usize {
        self.sb_indices.heap_bytes()
            + self.sb_classes.heap_bytes()
            + self.large_blocks.heap_bytes()
            + self.select_one_inds.heap_bytes()
            + self.select_zero_inds.heap_bytes()
    }
}

#[derive(Debug, Eq, PartialEq)]
struct LargeBlock {
    pointer: u64,
    rank: u64,
}

impl SpaceUsage for LargeBlock {
    fn is_stack_only() -> bool {
        true
    }

    fn heap_bytes(&self) -> usize {
        0
    }
}

#[derive(Debug, Eq, PartialEq)]
struct VarintBuffer {
    buf: Vec<u64>,
    len: usize,
}

impl VarintBuffer {
    fn with_capacity(bits: usize) -> Self {
        Self {
            buf: Vec::with_capacity(bits / 64),
            len: 0,
        }
    }

    fn push(&mut self, num_bits: usize, value: u64) {
        debug_assert!(num_bits <= 64);
        if num_bits == 0 {
            return;
        }
        let (block, offset) = (self.len / 64, self.len % 64);
        if self.buf.len() == block || offset + num_bits > 64 {
            self.buf.push(0);
        }
        self.buf[block] |= value << offset;
        if offset + num_bits > 64 {
            self.buf[block + 1] |= value >> (64 - offset);
        }
        self.len += num_bits;
    }

    fn get(&self, index: usize, num_bits: usize) -> u64 {
        debug_assert!(num_bits <= 64);
        if num_bits == 0 {
            return 0;
        }
        let (block, offset) = (index / 64, index % 64);
        let mask = 1u64
            .checked_shl(num_bits as u32)
            .unwrap_or(0)
            .wrapping_sub(1);
        let mut ret = (self.buf[block] >> offset) & mask;
        if offset + num_bits > 64 {
            ret |= self.buf[block + 1] << (64 - offset);
        }
        ret & mask
    }

    fn len(&self) -> usize {
        self.len
    }
}

impl SpaceUsage for VarintBuffer {
    fn is_stack_only() -> bool {
        false
    }

    fn heap_bytes(&self) -> usize {
        self.buf.heap_bytes()
    }
}

#[derive(Debug, Eq, PartialEq)]
struct LastBlock {
    bits: u64,
    num_ones: u64,
    num_zeros: u64,
}

impl LastBlock {
    fn new() -> Self {
        LastBlock {
            bits: 0,
            num_ones: 0,
            num_zeros: 0,
        }
    }

    fn select0(&self, rank: u8) -> u64 {
        debug_assert!(rank < self.num_zeros as u8);
        enum_code::select1_raw(!self.bits, rank as u64)
    }

    fn select1(&self, rank: u8) -> u64 {
        debug_assert!(rank < self.num_ones as u8);
        enum_code::select1_raw(self.bits, rank as u64)
    }

    // Count the number of bits set at indices i >= pos
    fn count_suffix(&self, pos: u64) -> u64 {
        (self.bits >> pos).count_ones() as u64
    }

    fn get_bit(&self, pos: u64) -> bool {
        (self.bits >> pos) & 1 == 1
    }

    // Only call one of `set_one` or `set_zeros` for any `pos`.
    fn set_one(&mut self, pos: u64) {
        self.bits |= 1 << pos;
        self.num_ones += 1;
    }
    fn set_zero(&mut self, _pos: u64) {
        self.num_zeros += 1;
    }
}

fn rank_by_bit(x: u64, n: u64, b: bool) -> u64 {
    if b {
        x
    } else {
        n - x
    }
}

#[cfg(test)]
mod tests {
    use super::RsDict;
    use crate::rank::RankSupport;
    use crate::rsdict::test_helpers::hash_u64;
    use crate::select::SelectSupport;

    // Ask quickcheck to generate blocks of 64 bits so we get test
    // coverage for ranges spanning multiple small blocks.
    fn test_rsdict(blocks: Vec<u64>) -> (Vec<bool>, RsDict) {
        let mut bits = Vec::with_capacity(blocks.len() * 64);
        let to_pop = blocks.get(0).unwrap_or(&0) % 64;
        for block in blocks {
            for i in 0..4 {
                let block = hash_u64(block.wrapping_add(i));
                if block % 2 != 0 {
                    for j in 0..64 {
                        let bit = (block >> j) & 1 != 0;
                        bits.push(bit);
                    }
                }
            }
        }
        for _ in 0..to_pop {
            bits.pop();
        }
        let mut rs_dict = RsDict::with_capacity(bits.len());
        for &bit in &bits {
            rs_dict.push(bit);
        }
        (bits, rs_dict)
    }

    #[quickcheck]
    fn qc_from_blocks(blocks: Vec<u64>) {
        let (bits, rs_dict) = test_rsdict(blocks);
        let blocks = (0..(bits.len() / 64)).map(|i| {
            let mut block = 0u64;
            for j in 0..64 {
                if bits[i * 64 + j] {
                    block |= 1 << j;
                }
            }
            block
        });
        let mut block_rs_dict = RsDict::from_blocks(blocks);
        for i in (bits.len() / 64 * 64)..bits.len() {
            block_rs_dict.push(bits[i]);
        }

        assert_eq!(rs_dict.len, block_rs_dict.len);
        assert_eq!(rs_dict.num_ones, block_rs_dict.num_ones);
        assert_eq!(rs_dict.num_zeros, block_rs_dict.num_zeros);
        assert_eq!(rs_dict.sb_classes, block_rs_dict.sb_classes);
        assert_eq!(rs_dict.sb_indices, block_rs_dict.sb_indices);
        assert_eq!(rs_dict.large_blocks, block_rs_dict.large_blocks);
        assert_eq!(rs_dict.select_one_inds, block_rs_dict.select_one_inds);
        assert_eq!(rs_dict.select_zero_inds, block_rs_dict.select_zero_inds);
        assert_eq!(rs_dict.last_block, block_rs_dict.last_block);
    }

    #[quickcheck]
    fn qc_rank(blocks: Vec<u64>) {
        let (bits, rs_dict) = test_rsdict(blocks);

        let mut one_rank = 0;
        let mut zero_rank = 0;

        // Check that rank(i) matches our naively computed ranks for all indices
        for (i, &inp_bit) in bits.iter().enumerate() {
            assert_eq!(rs_dict.rank(i as u64, false), zero_rank);
            assert_eq!(rs_dict.rank(i as u64, true), one_rank);
            if inp_bit {
                one_rank += 1;
            } else {
                zero_rank += 1;
            }
        }
    }

    #[quickcheck]
    fn qc_select(blocks: Vec<u64>) {
        let (bits, rs_dict) = test_rsdict(blocks);

        let mut one_rank = 0usize;
        let mut zero_rank = 0usize;

        // Check `select(r)` for ranks "in bounds" within the bitvector against
        // our naively computed ranks.
        for (i, &inp_bit) in bits.iter().enumerate() {
            if inp_bit {
                assert_eq!(rs_dict.select(one_rank as u64, true), Some(i as u64));
                one_rank += 1;
            } else {
                assert_eq!(rs_dict.select(zero_rank as u64, false), Some(i as u64));
                zero_rank += 1;
            }
        }
        // Check all of the "out of bounds" ranks up until `bits.len()`
        for r in (one_rank + 1)..bits.len() {
            assert_eq!(rs_dict.select(r as u64, true), None);
        }
        for r in (zero_rank + 1)..bits.len() {
            assert_eq!(rs_dict.select(r as u64, false), None);
        }
    }

    #[quickcheck]
    fn qc_get_bit(blocks: Vec<u64>) {
        let (bits, rs_dict) = test_rsdict(blocks);
        for (i, &bit) in bits.iter().enumerate() {
            assert_eq!(rs_dict.get_bit(i as u64), bit);
        }
    }

    #[quickcheck]
    fn qc_bit_and_one_rank(blocks: Vec<u64>) {
        let mut one_rank = 0;
        let (bits, rs_dict) = test_rsdict(blocks);
        for (i, &bit) in bits.iter().enumerate() {
            let (rs_bit, rs_rank) = rs_dict.bit_and_one_rank(i as u64);
            assert_eq!((rs_bit, rs_rank), (bit, one_rank));
            if bit {
                one_rank += 1;
            }
        }
    }
}

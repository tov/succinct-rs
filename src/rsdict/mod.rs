use std::mem;

mod constants;
mod enum_code;
mod helpers;

use super::rank::{
    RankSupport,
    BitRankSupport,
};
use super::select::{
    SelectSupport,
    Select1Support,
    Select0Support,
};
use super::space_usage::SpaceUsage;
use super::stream::{
    BitBuffer,
    BitWrite,
};
use super::bit_vec::{
    BitVec,
    BitVector,
};
use super::broadword;

use self::constants::*;
use self::enum_code::*;
use self::helpers::*;

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
    sb_indices: BitBuffer<BitVector<u64>>,

    // Large block metadata (stored every LARGE_BLOCK_SIZE bits):
    // * pointer into variable-length `bits` for the current index
    // * cached rank at the current index
    lb_pointers: Vec<u64>,
	lb_ranks: Vec<u64>,

    // Select acceleration metadata:
    // `select_{one,zero}_inds` store the (index / LARGE_BLOCK_SIZE) of each
    // SELECT_BLOCK_SIZE'th bit.
	select_one_inds: Vec<u64>,
	select_zero_inds: Vec<u64>,

    // Current in-progress small block we're appending to
    last_block: LastBlock,
}

#[derive(Debug)]
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
        let result = broadword::select1_raw(rank as usize, !self.bits);
        debug_assert_ne!(result, 72);
        result as u64
    }

    fn select1(&self, rank: u8) -> u64 {
        debug_assert!(rank < self.num_ones as u8);
        let result = broadword::select1_raw(rank as usize, self.bits);
        debug_assert_ne!(result, 72);
        result as u64
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

impl RankSupport for RsDict {
    type Over = bool;

    fn rank(&self, pos: u64, bit: bool) -> u64 {
        if pos >= self.len {
            return bit_num(self.num_ones, self.len, bit);
        }
        if self.is_last_block(pos) {
            let trailing_ones = self.last_block.count_suffix(pos % SMALL_BLOCK_SIZE);
            return bit_num(self.num_ones - trailing_ones, pos, bit);
        }
        let lblock = pos / LARGE_BLOCK_SIZE;
        let mut pointer = self.lb_pointers[lblock as usize];
        let sblock = pos / SMALL_BLOCK_SIZE;
        let mut rank = self.lb_ranks[lblock as usize];
        for i in (lblock * SMALL_BLOCK_PER_LARGE_BLOCK)..sblock {
            let sb_class = self.sb_classes[i as usize];
            pointer += ENUM_CODE_LENGTH[sb_class as usize] as u64;
            rank += sb_class as u64;
        }
        if pos % SMALL_BLOCK_SIZE == 0 {
            return bit_num(rank, pos, bit);
        }
        let sb_class = self.sb_classes[sblock as usize];
        let code = self.read_sb_index(pointer, ENUM_CODE_LENGTH[sb_class as usize]);
        rank += enum_rank(code, sb_class, (pos % SMALL_BLOCK_SIZE) as u8) as u64;
        bit_num(rank, pos, bit)
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
        if bit { self.select1(rank) } else { self.select0(rank) }
    }
}

impl Select0Support for RsDict {
    fn select0(&self, rank: u64) -> Option<u64> {
        if rank >= self.num_zeros {
            return None;
        }
        // How many zeros are there *excluding* the last block?
        let prefix_num_zeros = self.num_zeros - self.last_block.num_zeros;

        // Our index must be in the last block.
        if rank >= prefix_num_zeros {
            let lb_rank = (rank - prefix_num_zeros) as u8;
            return Some(self.last_block_ind() + self.last_block.select0(lb_rank));
        }

        let select_ind = rank / SELECT_BLOCK_SIZE;
        let mut lblock = self.select_zero_inds[select_ind as usize];
        while lblock < self.lb_ranks.len() as u64 {
            if rank < lblock * LARGE_BLOCK_SIZE - self.lb_ranks[lblock as usize] {
                break;
            }
            lblock += 1;
        }
        lblock -= 1;

        let mut sblock = lblock * SMALL_BLOCK_PER_LARGE_BLOCK;
        let mut pointer = self.lb_pointers[lblock as usize];
        let mut remain = rank - lblock * LARGE_BLOCK_SIZE + self.lb_ranks[lblock as usize] + 1;

        while sblock < self.sb_classes.len() as u64 {
            let sb_class = self.sb_classes[sblock as usize];
            let sb_zeros = SMALL_BLOCK_SIZE as u8 - sb_class;
            if remain <= sb_zeros as u64 {
                break;
            }
            remain -= sb_zeros as u64;
            pointer += ENUM_CODE_LENGTH[sb_zeros as usize] as u64;
            sblock += 1;
        }
        let sb_class = self.sb_classes[sblock as usize];
        let code = self.read_sb_index(pointer, ENUM_CODE_LENGTH[sb_class as usize]);
        Some(sblock * SMALL_BLOCK_SIZE + enum_select0(code, sb_class, remain as u8) as u64)
    }
}

impl Select1Support for RsDict {
    fn select1(&self, rank: u64) -> Option<u64> {
        if rank >= self.num_ones {
            return None;
        }
        // How many ones are there *excluding* the last block?
        let prefix_num_ones = self.num_ones - self.last_block.num_ones;

        // Our index must be in the last block.
        if rank >= prefix_num_ones {
            let lb_rank = (rank - prefix_num_ones) as u8;
            return Some(self.last_block_ind() + self.last_block.select1(lb_rank));
        }

        let select_ind = rank / SELECT_BLOCK_SIZE;
        let mut lblock = self.select_one_inds[select_ind as usize];

        while lblock < self.lb_ranks.len() as u64 {
            if rank < self.lb_ranks[lblock as usize] {
                break;
            }
            lblock += 1;
        }
        lblock -= 1;

        let mut sblock = lblock * SMALL_BLOCK_PER_LARGE_BLOCK;
        let mut pointer = self.lb_pointers[lblock as usize];
        let mut remain = rank - self.lb_ranks[lblock as usize] + 1;

        while sblock < self.sb_classes.len() as u64 {
            let sb_class = self.sb_classes[sblock as usize];
            if remain <= sb_class as u64 {
                break;
            }
            remain -= sb_class as u64;
            pointer += ENUM_CODE_LENGTH[sb_class as usize] as u64;

            sblock += 1;
        }
        let sb_class = self.sb_classes[sblock as usize];
        let code = self.read_sb_index(pointer, ENUM_CODE_LENGTH[sb_class as usize]);
        Some(sblock * SMALL_BLOCK_SIZE + enum_select1(code, sb_class, remain as u8) as u64)
    }
}

impl RsDict {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    fn with_capacity(n: usize) -> Self {
        Self {
            lb_pointers: Vec::with_capacity(n / LARGE_BLOCK_SIZE as usize),
            lb_ranks: Vec::with_capacity(n / LARGE_BLOCK_SIZE as usize),
            select_one_inds: Vec::with_capacity(n / SELECT_BLOCK_SIZE as usize),
            select_zero_inds: Vec::with_capacity(n / SELECT_BLOCK_SIZE as usize),
            sb_classes: Vec::with_capacity(n / SMALL_BLOCK_SIZE as usize),
            sb_indices: BitBuffer::with_capacity(n as u64 / SMALL_BLOCK_SIZE),

            len: 0,
            num_ones: 0,
            num_zeros: 0,

            last_block: LastBlock::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn count_ones(&self) -> usize {
        self.num_ones as usize
    }

    pub fn count_zeros(&self) -> usize {
        self.num_zeros as usize
    }

    pub fn push(&mut self, bit: bool) {
        if self.len % SMALL_BLOCK_SIZE == 0 {
            self.write_block();
        }
        if bit {
            self.last_block.set_one(self.len % SMALL_BLOCK_SIZE);
            if self.num_ones % SELECT_BLOCK_SIZE == 0 {
                // FIXME: This should be a vec of 54 bit indices.
                self.select_one_inds.push(self.len / LARGE_BLOCK_SIZE);
            }
            self.num_ones += 1;
        } else {
            self.last_block.set_zero(self.len % SMALL_BLOCK_SIZE);
            if self.num_zeros % SELECT_BLOCK_SIZE == 0 {
                // FIXME: This should be a vec of 54 bit indices.
                self.select_zero_inds.push(self.len / LARGE_BLOCK_SIZE);
            }
            self.num_zeros += 1;
        }
        self.len += 1;
    }

    pub fn get_bit(&self, pos: u64) -> bool {
        if self.is_last_block(pos) {
            return self.last_block.get_bit(pos % SMALL_BLOCK_SIZE);
        }
        let lblock = pos / LARGE_BLOCK_SIZE;
        let mut pointer = self.lb_pointers[lblock as usize]; // FIXME: get unchecked?
        let sblock = pos / SMALL_BLOCK_SIZE;

        for i in (lblock * SMALL_BLOCK_PER_LARGE_BLOCK)..sblock {
            let sb_class = self.sb_classes[i as usize];
            pointer += ENUM_CODE_LENGTH[sb_class as usize] as u64;
        }
        let sb_class = self.sb_classes[sblock as usize];
        let code = self.read_sb_index(pointer, ENUM_CODE_LENGTH[sb_class as usize]);
        enum_bit(code, sb_class, (pos % SMALL_BLOCK_SIZE) as u8)
    }

    pub fn bit_and_rank(&self, pos: u64) -> (bool, u64) {
        if self.is_last_block(pos) {
            let offset = pos % SMALL_BLOCK_SIZE;
            let bit = self.last_block.get_bit(offset);
            let after_rank = self.last_block.count_suffix(offset);
            return (bit, bit_num(self.num_ones - after_rank, pos, bit));
        }
        let lblock = pos / LARGE_BLOCK_SIZE;
        let mut pointer = self.lb_pointers[lblock as usize];
        let sblock = pos / SMALL_BLOCK_SIZE;
        let mut rank = self.lb_ranks[lblock as usize];
        for i in (lblock * SMALL_BLOCK_PER_LARGE_BLOCK)..sblock {
            let sb_class = self.sb_classes[i as usize];
            pointer += ENUM_CODE_LENGTH[sb_class as usize] as u64;
            rank += sb_class as u64;
        }
        let sb_class = self.sb_classes[sblock as usize];
        let code = self.read_sb_index(pointer, ENUM_CODE_LENGTH[sb_class as usize]);
        rank += enum_rank(code, sb_class, (pos % SMALL_BLOCK_SIZE) as u8) as u64;
        let bit = enum_bit(code, sb_class, (pos % SMALL_BLOCK_SIZE) as u8);
        (bit, bit_num(rank, pos, bit))
    }
}

impl RsDict {
    fn write_block(&mut self) {
        if self.len > 0 {
            let block = mem::replace(&mut self.last_block, LastBlock::new());

            let sb_class = block.num_ones as u8;
            self.sb_classes.push(sb_class);

            let code_len = ENUM_CODE_LENGTH[sb_class as usize];
            let code = enum_encode(block.bits, sb_class);

            // FIXME: This isn't specialized to write the integer all at once.
            self.sb_indices.write_int(code_len as usize, code)
                .expect("Developer error: write_int failed");
        }
        if self.len % LARGE_BLOCK_SIZE == 0 {
            self.lb_ranks.push(self.num_ones);
            self.lb_pointers.push(self.sb_bit_len());
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
        self.sb_indices.inner().get_bits(ptr, code_len as usize)
    }

    fn sb_bit_len(&self) -> u64 {
        self.sb_indices.inner().bit_len()
    }
}

impl SpaceUsage for RsDict {
    fn is_stack_only() -> bool {
        false
    }

    fn heap_bytes(&self) -> usize {
        self.sb_indices.inner().heap_bytes() +
            self.sb_classes.heap_bytes() +
            self.lb_pointers.heap_bytes() +
            self.lb_ranks.heap_bytes() +
            self.select_one_inds.heap_bytes() +
            self.select_zero_inds.heap_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::RsDict;
    use crate::rank::RankSupport;
    use crate::select::SelectSupport;

    #[quickcheck]
    fn rank_matches_simple(bits: Vec<bool>) -> bool {
        let mut rs_dict = RsDict::with_capacity(bits.len());
        for &bit in &bits {
            rs_dict.push(bit);
        }

        let mut one_rank = 0;
        let mut zero_rank = 0;

        // Check that rank(i) matches our naively computed ranks for all indices
        for (i, &inp_bit) in bits.iter().enumerate() {
            if rs_dict.rank(i as u64, false) != zero_rank {
                return false;
            }
            if rs_dict.rank(i as u64, true) != one_rank {
                return false;
            }
            if inp_bit {
                one_rank += 1;
            } else {
                zero_rank += 1;
            }
        }

        true
    }

    #[quickcheck]
    fn select_matches_simple(bits: Vec<bool>) -> bool {
        let mut rs_dict = RsDict::with_capacity(bits.len());
        for &bit in &bits {
            rs_dict.push(bit);
        }

        let mut one_rank = 0usize;
        let mut zero_rank = 0usize;

        // Check `select(r)` for ranks "in bounds" within the bitvector against
        // our naively computed ranks.
        for (i, &inp_bit) in bits.iter().enumerate() {
            if inp_bit {
                if rs_dict.select(one_rank as u64, true) != Some(i as u64) {
                    return false;
                }
                one_rank += 1;
            } else {
                if rs_dict.select(zero_rank as u64, false) != Some(i as u64) {
                    return false;
                }
                zero_rank += 1;
            }
        }
        // Check all of the "out of bounds" ranks up until `bits.len()`
        for r in (one_rank + 1)..bits.len() {
            if rs_dict.select(r as u64, true).is_some() {
                return false;
            }
        }
        for r in (zero_rank + 1)..bits.len() {
            if rs_dict.select(r as u64, false).is_some() {
                return false;
            }
        }
        true
    }
}
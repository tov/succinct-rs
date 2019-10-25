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

use self::constants::*;
use self::enum_code::*;
use self::helpers::*;

#[derive(Debug)]
pub struct RsDict {
    bits: Vec<u64>,
    pointer_blocks: Vec<u64>,
	rank_blocks: Vec<u64>,
	select_one_inds: Vec<u64>,
	select_zero_inds: Vec<u64>,
	rank_small_blocks: Vec<u8>,
	num: u64,
	one_num: u64,
	zero_num: u64,
	last_block: u64,
	last_one_num: u64,
	last_zero_num: u64,
	code_len: u64,
}

impl RankSupport for RsDict {
    type Over = bool;

    fn rank(&self, pos: u64, bit: bool) -> u64 {
        if pos >= self.num {
            return bit_num(self.one_num, self.num, bit);
        }
        if self.is_last_block(pos) {
            let after_rank = pop_count(self.last_block >> (pos % SMALL_BLOCK_SIZE));
            return bit_num(self.one_num - after_rank as u64, pos, bit);
        }
        let lblock = pos / LARGE_BLOCK_SIZE;
        let mut pointer = self.pointer_blocks[lblock as usize];
        let sblock = pos / SMALL_BLOCK_SIZE;
        let mut rank = self.rank_blocks[lblock as usize];
        for i in (lblock * SMALL_BLOCK_PER_LARGE_BLOCK)..sblock {
            let rank_sb = self.rank_small_blocks[i as usize];
            pointer += ENUM_CODE_LENGTH[rank_sb as usize] as u64;
            rank += rank_sb as u64;
        }
        if pos % SMALL_BLOCK_SIZE == 0 {
            return bit_num(rank, pos, bit);
        }
        let rank_sb = self.rank_small_blocks[sblock as usize];
        let code = get_slice(&self.bits, pointer, ENUM_CODE_LENGTH[rank_sb as usize]);
        rank += enum_rank(code, rank_sb, (pos % SMALL_BLOCK_SIZE) as u8) as u64;
        bit_num(rank, pos, bit)
    }

    fn limit(&self) -> u64 {
        self.num
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
        if rank >= self.zero_num {
            return None;
        }
        if rank >= self.zero_num - self.last_zero_num {
            let last_block_rank = (rank - (self.zero_num - self.last_zero_num)) as u8;
            return Some(self.last_block_ind() + select_raw(!self.last_block, last_block_rank + 1) as u64);
        }

        let select_ind = rank / SELECT_BLOCK_SIZE;
        let mut lblock = self.select_zero_inds[select_ind as usize];
        while lblock < self.rank_blocks.len() as u64 {
            if rank < lblock * LARGE_BLOCK_SIZE - self.rank_blocks[lblock as usize] {
                break;
            }
            lblock += 1;
        }
        lblock -= 1;

        let mut sblock = lblock * SMALL_BLOCK_PER_LARGE_BLOCK;
        let mut pointer = self.pointer_blocks[lblock as usize];
        let mut remain = rank - lblock * LARGE_BLOCK_SIZE + self.rank_blocks[lblock as usize] + 1;

        while sblock < self.rank_small_blocks.len() as u64 {
            let rank_sb = SMALL_BLOCK_SIZE as u8 - self.rank_small_blocks[sblock as usize];
            if remain <= rank_sb as u64 {
                break;
            }
            remain -= rank_sb as u64;
            pointer += ENUM_CODE_LENGTH[rank_sb as usize] as u64;
            sblock += 1;
        }
        let rank_sb = self.rank_small_blocks[sblock as usize];
        let code = get_slice(&self.bits, pointer, ENUM_CODE_LENGTH[rank_sb as usize]);
        Some(sblock * SMALL_BLOCK_SIZE + enum_select0(code, rank_sb, remain as u8) as u64)
    }
}

impl Select1Support for RsDict {
    fn select1(&self, rank: u64) -> Option<u64> {
        if rank >= self.one_num {
            return None;
        }
        if rank >= self.one_num - self.last_one_num {
            let last_block_rank = (rank - (self.one_num - self.last_one_num)) as u8;
            return Some(self.last_block_ind() + select_raw(self.last_block, last_block_rank+ 1) as u64);
        }

        let select_ind = rank / SELECT_BLOCK_SIZE;
        let mut lblock = self.select_one_inds[select_ind as usize];

        while lblock < self.rank_blocks.len() as u64 {
            if rank < self.rank_blocks[lblock as usize] {
                break;
            }
            lblock += 1;
        }
        lblock -= 1;

        let mut sblock = lblock * SMALL_BLOCK_PER_LARGE_BLOCK;
        let mut pointer = self.pointer_blocks[lblock as usize];
        let mut remain = rank - self.rank_blocks[lblock as usize] + 1;

        while sblock < self.rank_small_blocks.len() as u64 {
            let rank_sb = self.rank_small_blocks[sblock as usize];
            if remain <= rank_sb as u64 {
                break;
            }
            remain -= rank_sb as u64;
            pointer += ENUM_CODE_LENGTH[rank_sb as usize] as u64;

            sblock += 1;
        }
        let rank_sb = self.rank_small_blocks[sblock as usize];
        let code = get_slice(&self.bits, pointer, ENUM_CODE_LENGTH[rank_sb as usize]);
        Some(sblock * SMALL_BLOCK_SIZE + enum_select1(code, rank_sb, remain as u8) as u64)
    }
}

impl RsDict {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    fn with_capacity(n: usize) -> Self {
        Self {
            bits: Vec::with_capacity(n / SMALL_BLOCK_SIZE as usize),
            pointer_blocks: Vec::with_capacity(n / LARGE_BLOCK_SIZE as usize),
            rank_blocks: Vec::with_capacity(n / LARGE_BLOCK_SIZE as usize),
            select_one_inds: Vec::with_capacity(n / SELECT_BLOCK_SIZE as usize),
            select_zero_inds: Vec::with_capacity(n / SELECT_BLOCK_SIZE as usize),
            rank_small_blocks: Vec::with_capacity(n / SMALL_BLOCK_SIZE as usize),

            num: 0,
            one_num: 0,
            zero_num: 0,
            last_block: 0,
            last_one_num: 0,
            last_zero_num: 0,
            code_len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.num as usize
    }

    pub fn count_ones(&self) -> usize {
        self.one_num as usize
    }

    pub fn count_zeros(&self) -> usize {
        self.zero_num as usize
    }

    pub fn push(&mut self, bit: bool) {
        if self.num % SMALL_BLOCK_SIZE == 0 {
            self.write_block();
        }
        if bit {
            self.last_block |= 1 << (self.num % SMALL_BLOCK_SIZE);
            if self.one_num % SELECT_BLOCK_SIZE == 0 {
                self.select_one_inds.push(self.num / LARGE_BLOCK_SIZE);
            }
            self.one_num += 1;
            self.last_one_num += 1;
        } else {
            if self.zero_num % SELECT_BLOCK_SIZE == 0 {
                self.select_zero_inds.push(self.num / LARGE_BLOCK_SIZE);
            }
            self.zero_num += 1;
            self.last_zero_num += 1;
        }
        self.num += 1;
    }

    pub fn get_bit(&self, pos: u64) -> bool {
        if self.is_last_block(pos) {
            return get_bit(self.last_block, (pos % SMALL_BLOCK_SIZE) as u8);
        }
        let lblock = pos / LARGE_BLOCK_SIZE;
        let mut pointer = self.pointer_blocks[lblock as usize]; // FIXME: get unchecked?
        let sblock = pos / SMALL_BLOCK_SIZE;

        for i in (lblock * SMALL_BLOCK_PER_LARGE_BLOCK)..sblock {
            pointer += ENUM_CODE_LENGTH[self.rank_small_blocks[i as usize] as usize] as u64;
        }
        let rank_sb = self.rank_small_blocks[sblock as usize];
        let code = get_slice(&self.bits, pointer, ENUM_CODE_LENGTH[rank_sb as usize]);
        enum_bit(code, rank_sb, (pos % SMALL_BLOCK_SIZE) as u8)
    }


    pub fn bit_and_rank(&self, pos: u64) -> (bool, u64) {
        if self.is_last_block(pos) {
            let offset = (pos % SMALL_BLOCK_SIZE) as u8;
            let bit = get_bit(self.last_block, offset);
            let after_rank = pop_count(self.last_block >> offset) as u64;
            return (bit, bit_num(self.one_num - after_rank, pos, bit));
        }
        let lblock = pos / LARGE_BLOCK_SIZE;
        let mut pointer = self.pointer_blocks[lblock as usize];
        let sblock = pos / SMALL_BLOCK_SIZE;
        let mut rank = self.rank_blocks[lblock as usize];
        for i in (lblock * SMALL_BLOCK_PER_LARGE_BLOCK)..sblock {
            let rank_sb = self.rank_small_blocks[i as usize];
            pointer += ENUM_CODE_LENGTH[rank_sb as usize] as u64;
            rank += rank_sb as u64;
        }
        let rank_sb = self.rank_small_blocks[sblock as usize];
        let code = get_slice(&self.bits, pointer, ENUM_CODE_LENGTH[rank_sb as usize]);
        rank += enum_rank(code, rank_sb, (pos % SMALL_BLOCK_SIZE) as u8) as u64;
        let bit = enum_bit(code, rank_sb, (pos % SMALL_BLOCK_SIZE) as u8);
        (bit, bit_num(rank, pos, bit))
    }
}

impl RsDict {
    fn write_block(&mut self) {
        if self.num > 0 {
            let rank_sb = self.last_one_num as u8;
            self.rank_small_blocks.push(rank_sb);
            let code_len = ENUM_CODE_LENGTH[rank_sb as usize];
            let code = enum_encode(self.last_block, rank_sb);
            let new_size = floor(self.code_len + code_len as u64, SMALL_BLOCK_SIZE);
            if new_size > self.bits.len() as u64 {
                self.bits.push(0);
            }
            set_slice(&mut self.bits, self.code_len, code_len, code);
            self.last_block = 0;
            self.last_zero_num = 0;
            self.last_one_num = 0;
            self.code_len += code_len as u64;
        }
        if self.num % LARGE_BLOCK_SIZE == 0 {
            self.rank_blocks.push(self.one_num);
            self.pointer_blocks.push(self.code_len);
        }
    }

    fn last_block_ind(&self) -> u64 {
        if self.num == 0 {
            return 0;
        }
        ((self.num - 1) / SMALL_BLOCK_SIZE) * SMALL_BLOCK_SIZE
    }

    fn is_last_block(&self, pos: u64) -> bool {
        pos >= self.last_block_ind()
    }
}

impl SpaceUsage for RsDict {
    fn is_stack_only() -> bool {
        false
    }

    fn heap_bytes(&self) -> usize {
        self.bits.heap_bytes() +
            self.pointer_blocks.heap_bytes() +
            self.rank_blocks.heap_bytes() +
            self.select_one_inds.heap_bytes() +
            self.select_zero_inds.heap_bytes() +
            self.rank_small_blocks.heap_bytes()
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

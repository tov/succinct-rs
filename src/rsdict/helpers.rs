use super::constants::*;

// FIXME: get rid of most of these

pub fn floor(num: u64, div: u64) -> u64 {
    (num + div - 1) / div
}

pub fn decompose(x: u64, y: u64) -> (u64, u64) {
    (x / y, x % y)
}

pub fn set_slice(bits: &mut [u64], pos: u64, code_len: u8, val: u64) {
    if code_len == 0 {
        return;
    }
    let (block, offset) = decompose(pos, SMALL_BLOCK_SIZE);
    bits[block as usize] |= val << offset;
    if offset + (code_len as u64) > SMALL_BLOCK_SIZE {
        bits[block as usize + 1] |= val >> (SMALL_BLOCK_SIZE - offset);
    }
}

pub fn get_bit(block: u64, pos: u8) -> bool {
    ((block >> pos) & 1) == 1
}

pub fn get_slice(bits: &[u64], pos: u64, code_len: u8) -> u64 {
    if code_len == 0 {
        return 0;
    }
    let (block, offset) = decompose(pos, SMALL_BLOCK_SIZE);
    let mut ret = bits[block as usize] >> offset;

    if offset + (code_len as u64) > SMALL_BLOCK_SIZE {
        ret |= bits[block as usize + 1] << (SMALL_BLOCK_SIZE - offset);
    }
    if code_len == 64 {
        return ret;
    }
    ret & ((1 << code_len) - 1)
}

pub fn bit_num(x: u64, n: u64, b: bool) -> u64 {
    if b { x } else { n - x }
}

pub fn pop_count(x: u64) -> u8 {
    x.count_ones() as u8
}

pub fn select_raw(code: u64, mut rank: u8) -> u8 {
    for i in 0..SMALL_BLOCK_SIZE {
        if get_bit(code, i as u8) {
            rank -= 1;
            if rank == 0 {
                return i as u8;
            }
        }
    }
    0
}

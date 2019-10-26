use super::constants::*;

pub fn get_bit(block: u64, pos: u8) -> bool {
    ((block >> pos) & 1) == 1
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

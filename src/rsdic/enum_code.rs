use crate::broadword;
use super::constants::SMALL_BLOCK_SIZE;

pub fn encode(value: u64, class: u8) -> (u8, u64) {
    debug_assert_eq!(value.count_ones() as u8, class);
    let code_len = ENUM_CODE_LENGTH[class as usize];

    // Fast path: return the integer unchanged if we're using all of our bits.
    if code_len == SMALL_BLOCK_SIZE as u8 {
        return (code_len, value);
    }

    let mut code = 0u64;
    let mut k = class;
    for i in 0..(SMALL_BLOCK_SIZE as u8) {
        if (value >> i) & 1 != 0 {
            let n = (SMALL_BLOCK_SIZE as u8 - i) - 1;
            code += BINOMIAL_COEFFICIENTS[n as usize][k as usize];
            k -= 1;
        }
    }
    (code_len, code)
}

#[cfg(test)]
pub fn decode(mut code: u64, class: u8) -> u64 {
    if ENUM_CODE_LENGTH[class as usize] == SMALL_BLOCK_SIZE as u8 {
        return code;
    }
    let mut value = 0u64;
    let mut k = class;
    for i in 0..(SMALL_BLOCK_SIZE as u8) {
        let n = SMALL_BLOCK_SIZE as u8 - i - 1;
        let zero_case_num = BINOMIAL_COEFFICIENTS[n as usize][k as usize];
        if code >= zero_case_num {
            value |= 1 << i;
            code -= zero_case_num;
            k -= 1;
        }
    }
    value
}

pub fn decode_bit(mut code: u64, class: u8, pos: u64) -> bool {
    if ENUM_CODE_LENGTH[class as usize] == SMALL_BLOCK_SIZE as u8 {
        return (code >> pos) & 1 != 0
    }
    let mut k = class;
    for i in 0..(pos as u8) {
        let n = SMALL_BLOCK_SIZE as u8 - i - 1;
        let zero_case_num = BINOMIAL_COEFFICIENTS[n as usize][k as usize];
        if code >= zero_case_num {
            code -= zero_case_num;
            k -= 1;
        }
    }
    let n = SMALL_BLOCK_SIZE - pos - 1;
    code >= BINOMIAL_COEFFICIENTS[n as usize][k as usize]

}

pub fn rank(mut code: u64, class: u8, pos: u64) -> u64 {
    if ENUM_CODE_LENGTH[class as usize] == SMALL_BLOCK_SIZE as u8 {
        return (code & ((1 << pos) - 1)).count_ones() as u64
    }
    let mut cur_rank = class;
    for i in 0..pos {
        let n = SMALL_BLOCK_SIZE - i - 1;
        let zero_case_num = BINOMIAL_COEFFICIENTS[n as usize][cur_rank as usize];
        if code >= zero_case_num {
            code -= zero_case_num;
            cur_rank -= 1;
        }
    }
    (class - cur_rank) as u64
}

pub fn select1(mut code: u64, class: u8, mut rank: u64) -> u64 {
    if ENUM_CODE_LENGTH[class as usize] == SMALL_BLOCK_SIZE as u8 {
        let result = broadword::select1_raw(rank as usize, code);
        debug_assert_ne!(result, 72);
        return result as u64;
    }
    let mut k = class;
    for i in 0..SMALL_BLOCK_SIZE {
        let n = SMALL_BLOCK_SIZE - i - 1;
        let zero_case_num = BINOMIAL_COEFFICIENTS[n as usize][k as usize];
        if code >= zero_case_num {
            if rank == 0 {
                return i;
            }
            rank -= 1;
            code -= zero_case_num;
            k -= 1;
        }
    }
    debug_assert!(false, "select1 past end of codeword");
    0
}

pub fn select0(mut code: u64, class: u8, mut rank: u64) -> u64 {
    if ENUM_CODE_LENGTH[class as usize] == SMALL_BLOCK_SIZE as u8 {
        let result = broadword::select1_raw(rank as usize, !code);
        debug_assert_ne!(result, 72);
        return result as u64;
    }
    let mut k = class as usize;
    for i in 0..SMALL_BLOCK_SIZE {
        let n = SMALL_BLOCK_SIZE - i - 1;
        let zero_case_num = BINOMIAL_COEFFICIENTS[n as usize][k];
        if code >= zero_case_num {
            code -= zero_case_num;
            k -= 1;
        } else {
            if rank == 0 {
                return i;
            }
            rank -= 1;
        }
    }
    debug_assert!(false, "select0 past end of codeword");
    0
}

#[cfg(test)]
mod tests {
    use broadword;
    use super::{decode, decode_bit, encode, rank, select0, select1};

    #[quickcheck]
    fn qc_decode(value: u64) -> bool {
        let class = value.count_ones() as u8;
        let (_, code) = encode(value, class);
        decode(code, class) == value
    }

    #[quickcheck]
    fn qc_decode_bit(value: u64) -> bool {
        let class = value.count_ones() as u8;
        let (_, code) = encode(value, class);
        (0..64)
            .all(|i| {
                let computed = decode_bit(code, class, i);
                let expected = (value >> i) & 1 != 0;
                computed == expected
            })
    }

    #[quickcheck]
    fn qc_rank(value: u64) -> bool {
        let class = value.count_ones() as u8;
        let (_, code) = encode(value, class);
        (0..64)
            .all(|i| {
                let computed = rank(code, class, i);
                let expected = (value & ((1 << i) - 1)).count_ones() as u64;
                computed == expected
            })
    }

    #[quickcheck]
    fn qc_select0(value: u64) -> bool {
        let class = value.count_ones() as u8;
        let (_, code) = encode(value, class);
        (0..class as u64)
            .all(|i| {
                let computed = select0(code, class, i) as usize;
                let expected = broadword::select1_raw(i as usize, !value);
                computed == expected
            })
    }

    #[quickcheck]
    fn qc_select1(value: u64) -> bool {
        let class = value.count_ones() as u8;
        let (_, code) = encode(value, class);
        (0..class as u64)
            .all(|i| {
                let computed = select1(code, class, i) as usize;
                let expected = broadword::select1_raw(i as usize, value);
                computed == expected
            })
    }
}

// TODO: Generate this using `const fn` when it stabilizes.
// Precomputed table of binomial_coefficient(n, r) for n in [0, 64] and r in [0, 64]
const BINOMIAL_COEFFICIENTS: &[&[u64; 65]; 65] = &[
    &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 3, 3, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 4, 6, 4, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 5, 10, 10, 5, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 6, 15, 20, 15, 6, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 7, 21, 35, 35, 21, 7, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 8, 28, 56, 70, 56, 28, 8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 9, 36, 84, 126, 126, 84, 36, 9, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 10, 45, 120, 210, 252, 210, 120, 45, 10, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 11, 55, 165, 330, 462, 462, 330, 165, 55, 11, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 12, 66, 220, 495, 792, 924, 792, 495, 220, 66, 12, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 13, 78, 286, 715, 1287, 1716, 1716, 1287, 715, 286, 78, 13, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 14, 91, 364, 1001, 2002, 3003, 3432, 3003, 2002, 1001, 364, 91, 14, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 15, 105, 455, 1365, 3003, 5005, 6435, 6435, 5005, 3003, 1365, 455, 105, 15, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 16, 120, 560, 1820, 4368, 8008, 11440, 12870, 11440, 8008, 4368, 1820, 560, 120, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 17, 136, 680, 2380, 6188, 12376, 19448, 24310, 24310, 19448, 12376, 6188, 2380, 680, 136, 17, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 18, 153, 816, 3060, 8568, 18564, 31824, 43758, 48620, 43758, 31824, 18564, 8568, 3060, 816, 153, 18, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 19, 171, 969, 3876, 11628, 27132, 50388, 75582, 92378, 92378, 75582, 50388, 27132, 11628, 3876, 969, 171, 19, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 20, 190, 1140, 4845, 15504, 38760, 77520, 125970, 167960, 184756, 167960, 125970, 77520, 38760, 15504, 4845, 1140, 190, 20, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 21, 210, 1330, 5985, 20349, 54264, 116280, 203490, 293930, 352716, 352716, 293930, 203490, 116280, 54264, 20349, 5985, 1330, 210, 21, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 22, 231, 1540, 7315, 26334, 74613, 170544, 319770, 497420, 646646, 705432, 646646, 497420, 319770, 170544, 74613, 26334, 7315, 1540, 231, 22, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 23, 253, 1771, 8855, 33649, 100947, 245157, 490314, 817190, 1144066, 1352078, 1352078, 1144066, 817190, 490314, 245157, 100947, 33649, 8855, 1771, 253, 23, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 24, 276, 2024, 10626, 42504, 134596, 346104, 735471, 1307504, 1961256, 2496144, 2704156, 2496144, 1961256, 1307504, 735471, 346104, 134596, 42504, 10626, 2024, 276, 24, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 25, 300, 2300, 12650, 53130, 177100, 480700, 1081575, 2042975, 3268760, 4457400, 5200300, 5200300, 4457400, 3268760, 2042975, 1081575, 480700, 177100, 53130, 12650, 2300, 300, 25, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 26, 325, 2600, 14950, 65780, 230230, 657800, 1562275, 3124550, 5311735, 7726160, 9657700, 10400600, 9657700, 7726160, 5311735, 3124550, 1562275, 657800, 230230, 65780, 14950, 2600, 325, 26, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 27, 351, 2925, 17550, 80730, 296010, 888030, 2220075, 4686825, 8436285, 13037895, 17383860, 20058300, 20058300, 17383860, 13037895, 8436285, 4686825, 2220075, 888030, 296010, 80730, 17550, 2925, 351, 27, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 28, 378, 3276, 20475, 98280, 376740, 1184040, 3108105, 6906900, 13123110, 21474180, 30421755, 37442160, 40116600, 37442160, 30421755, 21474180, 13123110, 6906900, 3108105, 1184040, 376740, 98280, 20475, 3276, 378, 28, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 29, 406, 3654, 23751, 118755, 475020, 1560780, 4292145, 10015005, 20030010, 34597290, 51895935, 67863915, 77558760, 77558760, 67863915, 51895935, 34597290, 20030010, 10015005, 4292145, 1560780, 475020, 118755, 23751, 3654, 406, 29, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 30, 435, 4060, 27405, 142506, 593775, 2035800, 5852925, 14307150, 30045015, 54627300, 86493225, 119759850, 145422675, 155117520, 145422675, 119759850, 86493225, 54627300, 30045015, 14307150, 5852925, 2035800, 593775, 142506, 27405, 4060, 435, 30, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 31, 465, 4495, 31465, 169911, 736281, 2629575, 7888725, 20160075, 44352165, 84672315, 141120525, 206253075, 265182525, 300540195, 300540195, 265182525, 206253075, 141120525, 84672315, 44352165, 20160075, 7888725, 2629575, 736281, 169911, 31465, 4495, 465, 31, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 32, 496, 4960, 35960, 201376, 906192, 3365856, 10518300, 28048800, 64512240, 129024480, 225792840, 347373600, 471435600, 565722720, 601080390, 565722720, 471435600, 347373600, 225792840, 129024480, 64512240, 28048800, 10518300, 3365856, 906192, 201376, 35960, 4960, 496, 32, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 33, 528, 5456, 40920, 237336, 1107568, 4272048, 13884156, 38567100, 92561040, 193536720, 354817320, 573166440, 818809200, 1037158320, 1166803110, 1166803110, 1037158320, 818809200, 573166440, 354817320, 193536720, 92561040, 38567100, 13884156, 4272048, 1107568, 237336, 40920, 5456, 528, 33, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 34, 561, 5984, 46376, 278256, 1344904, 5379616, 18156204, 52451256, 131128140, 286097760, 548354040, 927983760, 1391975640, 1855967520, 2203961430, 2333606220, 2203961430, 1855967520, 1391975640, 927983760, 548354040, 286097760, 131128140, 52451256, 18156204, 5379616, 1344904, 278256, 46376, 5984, 561, 34, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 35, 595, 6545, 52360, 324632, 1623160, 6724520, 23535820, 70607460, 183579396, 417225900, 834451800, 1476337800, 2319959400, 3247943160, 4059928950, 4537567650, 4537567650, 4059928950, 3247943160, 2319959400, 1476337800, 834451800, 417225900, 183579396, 70607460, 23535820, 6724520, 1623160, 324632, 52360, 6545, 595, 35, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 36, 630, 7140, 58905, 376992, 1947792, 8347680, 30260340, 94143280, 254186856, 600805296, 1251677700, 2310789600, 3796297200, 5567902560, 7307872110, 8597496600, 9075135300, 8597496600, 7307872110, 5567902560, 3796297200, 2310789600, 1251677700, 600805296, 254186856, 94143280, 30260340, 8347680, 1947792, 376992, 58905, 7140, 630, 36, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 37, 666, 7770, 66045, 435897, 2324784, 10295472, 38608020, 124403620, 348330136, 854992152, 1852482996, 3562467300, 6107086800, 9364199760, 12875774670, 15905368710, 17672631900, 17672631900, 15905368710, 12875774670, 9364199760, 6107086800, 3562467300, 1852482996, 854992152, 348330136, 124403620, 38608020, 10295472, 2324784, 435897, 66045, 7770, 666, 37, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 38, 703, 8436, 73815, 501942, 2760681, 12620256, 48903492, 163011640, 472733756, 1203322288, 2707475148, 5414950296, 9669554100, 15471286560, 22239974430, 28781143380, 33578000610, 35345263800, 33578000610, 28781143380, 22239974430, 15471286560, 9669554100, 5414950296, 2707475148, 1203322288, 472733756, 163011640, 48903492, 12620256, 2760681, 501942, 73815, 8436, 703, 38, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 39, 741, 9139, 82251, 575757, 3262623, 15380937, 61523748, 211915132, 635745396, 1676056044, 3910797436, 8122425444, 15084504396, 25140840660, 37711260990, 51021117810, 62359143990, 68923264410, 68923264410, 62359143990, 51021117810, 37711260990, 25140840660, 15084504396, 8122425444, 3910797436, 1676056044, 635745396, 211915132, 61523748, 15380937, 3262623, 575757, 82251, 9139, 741, 39, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 40, 780, 9880, 91390, 658008, 3838380, 18643560, 76904685, 273438880, 847660528, 2311801440, 5586853480, 12033222880, 23206929840, 40225345056, 62852101650, 88732378800, 113380261800, 131282408400, 137846528820, 131282408400, 113380261800, 88732378800, 62852101650, 40225345056, 23206929840, 12033222880, 5586853480, 2311801440, 847660528, 273438880, 76904685, 18643560, 3838380, 658008, 91390, 9880, 780, 40, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 41, 820, 10660, 101270, 749398, 4496388, 22481940, 95548245, 350343565, 1121099408, 3159461968, 7898654920, 17620076360, 35240152720, 63432274896, 103077446706, 151584480450, 202112640600, 244662670200, 269128937220, 269128937220, 244662670200, 202112640600, 151584480450, 103077446706, 63432274896, 35240152720, 17620076360, 7898654920, 3159461968, 1121099408, 350343565, 95548245, 22481940, 4496388, 749398, 101270, 10660, 820, 41, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 42, 861, 11480, 111930, 850668, 5245786, 26978328, 118030185, 445891810, 1471442973, 4280561376, 11058116888, 25518731280, 52860229080, 98672427616, 166509721602, 254661927156, 353697121050, 446775310800, 513791607420, 538257874440, 513791607420, 446775310800, 353697121050, 254661927156, 166509721602, 98672427616, 52860229080, 25518731280, 11058116888, 4280561376, 1471442973, 445891810, 118030185, 26978328, 5245786, 850668, 111930, 11480, 861, 42, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 43, 903, 12341, 123410, 962598, 6096454, 32224114, 145008513, 563921995, 1917334783, 5752004349, 15338678264, 36576848168, 78378960360, 151532656696, 265182149218, 421171648758, 608359048206, 800472431850, 960566918220, 1052049481860, 1052049481860, 960566918220, 800472431850, 608359048206, 421171648758, 265182149218, 151532656696, 78378960360, 36576848168, 15338678264, 5752004349, 1917334783, 563921995, 145008513, 32224114, 6096454, 962598, 123410, 12341, 903, 43, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 44, 946, 13244, 135751, 1086008, 7059052, 38320568, 177232627, 708930508, 2481256778, 7669339132, 21090682613, 51915526432, 114955808528, 229911617056, 416714805914, 686353797976, 1029530696964, 1408831480056, 1761039350070, 2012616400080, 2104098963720, 2012616400080, 1761039350070, 1408831480056, 1029530696964, 686353797976, 416714805914, 229911617056, 114955808528, 51915526432, 21090682613, 7669339132, 2481256778, 708930508, 177232627, 38320568, 7059052, 1086008, 135751, 13244, 946, 44, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 45, 990, 14190, 148995, 1221759, 8145060, 45379620, 215553195, 886163135, 3190187286, 10150595910, 28760021745, 73006209045, 166871334960, 344867425584, 646626422970, 1103068603890, 1715884494940, 2438362177020, 3169870830126, 3773655750150, 4116715363800, 4116715363800, 3773655750150, 3169870830126, 2438362177020, 1715884494940, 1103068603890, 646626422970, 344867425584, 166871334960, 73006209045, 28760021745, 10150595910, 3190187286, 886163135, 215553195, 45379620, 8145060, 1221759, 148995, 14190, 990, 45, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 46, 1035, 15180, 163185, 1370754, 9366819, 53524680, 260932815, 1101716330, 4076350421, 13340783196, 38910617655, 101766230790, 239877544005, 511738760544, 991493848554, 1749695026860, 2818953098830, 4154246671960, 5608233007146, 6943526580276, 7890371113950, 8233430727600, 7890371113950, 6943526580276, 5608233007146, 4154246671960, 2818953098830, 1749695026860, 991493848554, 511738760544, 239877544005, 101766230790, 38910617655, 13340783196, 4076350421, 1101716330, 260932815, 53524680, 9366819, 1370754, 163185, 15180, 1035, 46, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 47, 1081, 16215, 178365, 1533939, 10737573, 62891499, 314457495, 1362649145, 5178066751, 17417133617, 52251400851, 140676848445, 341643774795, 751616304549, 1503232609098, 2741188875414, 4568648125690, 6973199770790, 9762479679106, 12551759587422, 14833897694226, 16123801841550, 16123801841550, 14833897694226, 12551759587422, 9762479679106, 6973199770790, 4568648125690, 2741188875414, 1503232609098, 751616304549, 341643774795, 140676848445, 52251400851, 17417133617, 5178066751, 1362649145, 314457495, 62891499, 10737573, 1533939, 178365, 16215, 1081, 47, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 48, 1128, 17296, 194580, 1712304, 12271512, 73629072, 377348994, 1677106640, 6540715896, 22595200368, 69668534468, 192928249296, 482320623240, 1093260079344, 2254848913647, 4244421484512, 7309837001104, 11541847896480, 16735679449896, 22314239266528, 27385657281648, 30957699535776, 32247603683100, 30957699535776, 27385657281648, 22314239266528, 16735679449896, 11541847896480, 7309837001104, 4244421484512, 2254848913647, 1093260079344, 482320623240, 192928249296, 69668534468, 22595200368, 6540715896, 1677106640, 377348994, 73629072, 12271512, 1712304, 194580, 17296, 1128, 48, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 49, 1176, 18424, 211876, 1906884, 13983816, 85900584, 450978066, 2054455634, 8217822536, 29135916264, 92263734836, 262596783764, 675248872536, 1575580702584, 3348108992991, 6499270398159, 11554258485616, 18851684897584, 28277527346376, 39049918716424, 49699896548176, 58343356817424, 63205303218876, 63205303218876, 58343356817424, 49699896548176, 39049918716424, 28277527346376, 18851684897584, 11554258485616, 6499270398159, 3348108992991, 1575580702584, 675248872536, 262596783764, 92263734836, 29135916264, 8217822536, 2054455634, 450978066, 85900584, 13983816, 1906884, 211876, 18424, 1176, 49, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 50, 1225, 19600, 230300, 2118760, 15890700, 99884400, 536878650, 2505433700, 10272278170, 37353738800, 121399651100, 354860518600, 937845656300, 2250829575120, 4923689695575, 9847379391150, 18053528883775, 30405943383200, 47129212243960, 67327446062800, 88749815264600, 108043253365600, 121548660036300, 126410606437752, 121548660036300, 108043253365600, 88749815264600, 67327446062800, 47129212243960, 30405943383200, 18053528883775, 9847379391150, 4923689695575, 2250829575120, 937845656300, 354860518600, 121399651100, 37353738800, 10272278170, 2505433700, 536878650, 99884400, 15890700, 2118760, 230300, 19600, 1225, 50, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 51, 1275, 20825, 249900, 2349060, 18009460, 115775100, 636763050, 3042312350, 12777711870, 47626016970, 158753389900, 476260169700, 1292706174900, 3188675231420, 7174519270695, 14771069086725, 27900908274925, 48459472266975, 77535155627160, 114456658306760, 156077261327400, 196793068630200, 229591913401900, 247959266474052, 247959266474052, 229591913401900, 196793068630200, 156077261327400, 114456658306760, 77535155627160, 48459472266975, 27900908274925, 14771069086725, 7174519270695, 3188675231420, 1292706174900, 476260169700, 158753389900, 47626016970, 12777711870, 3042312350, 636763050, 115775100, 18009460, 2349060, 249900, 20825, 1275, 51, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 52, 1326, 22100, 270725, 2598960, 20358520, 133784560, 752538150, 3679075400, 15820024220, 60403728840, 206379406870, 635013559600, 1768966344600, 4481381406320, 10363194502115, 21945588357420, 42671977361650, 76360380541900, 125994627894135, 191991813933920, 270533919634160, 352870329957600, 426384982032100, 477551179875952, 495918532948104, 477551179875952, 426384982032100, 352870329957600, 270533919634160, 191991813933920, 125994627894135, 76360380541900, 42671977361650, 21945588357420, 10363194502115, 4481381406320, 1768966344600, 635013559600, 206379406870, 60403728840, 15820024220, 3679075400, 752538150, 133784560, 20358520, 2598960, 270725, 22100, 1326, 52, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 53, 1378, 23426, 292825, 2869685, 22957480, 154143080, 886322710, 4431613550, 19499099620, 76223753060, 266783135710, 841392966470, 2403979904200, 6250347750920, 14844575908435, 32308782859535, 64617565719070, 119032357903550, 202355008436035, 317986441828055, 462525733568080, 623404249591760, 779255311989700, 903936161908052, 973469712824056, 973469712824056, 903936161908052, 779255311989700, 623404249591760, 462525733568080, 317986441828055, 202355008436035, 119032357903550, 64617565719070, 32308782859535, 14844575908435, 6250347750920, 2403979904200, 841392966470, 266783135710, 76223753060, 19499099620, 4431613550, 886322710, 154143080, 22957480, 2869685, 292825, 23426, 1378, 53, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 54, 1431, 24804, 316251, 3162510, 25827165, 177100560, 1040465790, 5317936260, 23930713170, 95722852680, 343006888770, 1108176102180, 3245372870670, 8654327655120, 21094923659355, 47153358767970, 96926348578605, 183649923622620, 321387366339585, 520341450264090, 780512175396135, 1085929983159840, 1402659561581460, 1683191473897752, 1877405874732108, 1946939425648112, 1877405874732108, 1683191473897752, 1402659561581460, 1085929983159840, 780512175396135, 520341450264090, 321387366339585, 183649923622620, 96926348578605, 47153358767970, 21094923659355, 8654327655120, 3245372870670, 1108176102180, 343006888770, 95722852680, 23930713170, 5317936260, 1040465790, 177100560, 25827165, 3162510, 316251, 24804, 1431, 54, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 55, 1485, 26235, 341055, 3478761, 28989675, 202927725, 1217566350, 6358402050, 29248649430, 119653565850, 438729741450, 1451182990950, 4353548972850, 11899700525790, 29749251314475, 68248282427325, 144079707346575, 280576272201225, 505037289962205, 841728816603675, 1300853625660225, 1866442158555975, 2488589544741300, 3085851035479212, 3560597348629860, 3824345300380220, 3824345300380220, 3560597348629860, 3085851035479212, 2488589544741300, 1866442158555975, 1300853625660225, 841728816603675, 505037289962205, 280576272201225, 144079707346575, 68248282427325, 29749251314475, 11899700525790, 4353548972850, 1451182990950, 438729741450, 119653565850, 29248649430, 6358402050, 1217566350, 202927725, 28989675, 3478761, 341055, 26235, 1485, 55, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 56, 1540, 27720, 367290, 3819816, 32468436, 231917400, 1420494075, 7575968400, 35607051480, 148902215280, 558383307300, 1889912732400, 5804731963800, 16253249498640, 41648951840265, 97997533741800, 212327989773900, 424655979547800, 785613562163430, 1346766106565880, 2142582442263900, 3167295784216200, 4355031703297275, 5574440580220512, 6646448384109072, 7384942649010080, 7648690600760440, 7384942649010080, 6646448384109072, 5574440580220512, 4355031703297275, 3167295784216200, 2142582442263900, 1346766106565880, 785613562163430, 424655979547800, 212327989773900, 97997533741800, 41648951840265, 16253249498640, 5804731963800, 1889912732400, 558383307300, 148902215280, 35607051480, 7575968400, 1420494075, 231917400, 32468436, 3819816, 367290, 27720, 1540, 56, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    &[1, 57, 1596, 29260, 395010, 4187106, 36288252, 264385836, 1652411475, 8996462475, 43183019880, 184509266760, 707285522580, 2448296039700, 7694644696200, 22057981462440, 57902201338905, 139646485582065, 310325523515700, 636983969321700, 1210269541711230, 2132379668729310, 3489348548829780, 5309878226480100, 7522327487513475, 9929472283517787, 12220888964329584, 14031391033119152, 15033633249770520, 15033633249770520, 14031391033119152, 12220888964329584, 9929472283517787, 7522327487513475, 5309878226480100, 3489348548829780, 2132379668729310, 1210269541711230, 636983969321700, 310325523515700, 139646485582065, 57902201338905, 22057981462440, 7694644696200, 2448296039700, 707285522580, 184509266760, 43183019880, 8996462475, 1652411475, 264385836, 36288252, 4187106, 395010, 29260, 1596, 57, 1, 0, 0, 0, 0, 0, 0, 0],
    &[1, 58, 1653, 30856, 424270, 4582116, 40475358, 300674088, 1916797311, 10648873950, 52179482355, 227692286640, 891794789340, 3155581562280, 10142940735900, 29752626158640, 79960182801345, 197548686920970, 449972009097765, 947309492837400, 1847253511032930, 3342649210440540, 5621728217559090, 8799226775309880, 12832205713993575, 17451799771031262, 22150361247847371, 26252279997448736, 29065024282889672, 30067266499541040, 29065024282889672, 26252279997448736, 22150361247847371, 17451799771031262, 12832205713993575, 8799226775309880, 5621728217559090, 3342649210440540, 1847253511032930, 947309492837400, 449972009097765, 197548686920970, 79960182801345, 29752626158640, 10142940735900, 3155581562280, 891794789340, 227692286640, 52179482355, 10648873950, 1916797311, 300674088, 40475358, 4582116, 424270, 30856, 1653, 58, 1, 0, 0, 0, 0, 0, 0],
    &[1, 59, 1711, 32509, 455126, 5006386, 45057474, 341149446, 2217471399, 12565671261, 62828356305, 279871768995, 1119487075980, 4047376351620, 13298522298180, 39895566894540, 109712808959985, 277508869722315, 647520696018735, 1397281501935165, 2794563003870330, 5189902721473470, 8964377427999630, 14420954992868970, 21631432489303455, 30284005485024837, 39602161018878633, 48402641245296107, 55317304280338408, 59132290782430712, 59132290782430712, 55317304280338408, 48402641245296107, 39602161018878633, 30284005485024837, 21631432489303455, 14420954992868970, 8964377427999630, 5189902721473470, 2794563003870330, 1397281501935165, 647520696018735, 277508869722315, 109712808959985, 39895566894540, 13298522298180, 4047376351620, 1119487075980, 279871768995, 62828356305, 12565671261, 2217471399, 341149446, 45057474, 5006386, 455126, 32509, 1711, 59, 1, 0, 0, 0, 0, 0],
    &[1, 60, 1770, 34220, 487635, 5461512, 50063860, 386206920, 2558620845, 14783142660, 75394027566, 342700125300, 1399358844975, 5166863427600, 17345898649800, 53194089192720, 149608375854525, 387221678682300, 925029565741050, 2044802197953900, 4191844505805495, 7984465725343800, 14154280149473100, 23385332420868600, 36052387482172425, 51915437974328292, 69886166503903470, 88004802264174740, 103719945525634515, 114449595062769120, 118264581564861424, 114449595062769120, 103719945525634515, 88004802264174740, 69886166503903470, 51915437974328292, 36052387482172425, 23385332420868600, 14154280149473100, 7984465725343800, 4191844505805495, 2044802197953900, 925029565741050, 387221678682300, 149608375854525, 53194089192720, 17345898649800, 5166863427600, 1399358844975, 342700125300, 75394027566, 14783142660, 2558620845, 386206920, 50063860, 5461512, 487635, 34220, 1770, 60, 1, 0, 0, 0, 0],
    &[1, 61, 1830, 35990, 521855, 5949147, 55525372, 436270780, 2944827765, 17341763505, 90177170226, 418094152866, 1742058970275, 6566222272575, 22512762077400, 70539987842520, 202802465047245, 536830054536825, 1312251244423350, 2969831763694950, 6236646703759395, 12176310231149295, 22138745874816900, 37539612570341700, 59437719903041025, 87967825456500717, 121801604478231762, 157890968768078210, 191724747789809255, 218169540588403635, 232714176627630544, 232714176627630544, 218169540588403635, 191724747789809255, 157890968768078210, 121801604478231762, 87967825456500717, 59437719903041025, 37539612570341700, 22138745874816900, 12176310231149295, 6236646703759395, 2969831763694950, 1312251244423350, 536830054536825, 202802465047245, 70539987842520, 22512762077400, 6566222272575, 1742058970275, 418094152866, 90177170226, 17341763505, 2944827765, 436270780, 55525372, 5949147, 521855, 35990, 1830, 61, 1, 0, 0, 0],
    &[1, 62, 1891, 37820, 557845, 6471002, 61474519, 491796152, 3381098545, 20286591270, 107518933731, 508271323092, 2160153123141, 8308281242850, 29078984349975, 93052749919920, 273342452889765, 739632519584070, 1849081298960175, 4282083008118300, 9206478467454345, 18412956934908690, 34315056105966195, 59678358445158600, 96977332473382725, 147405545359541742, 209769429934732479, 279692573246309972, 349615716557887465, 409894288378212890, 450883717216034179, 465428353255261088, 450883717216034179, 409894288378212890, 349615716557887465, 279692573246309972, 209769429934732479, 147405545359541742, 96977332473382725, 59678358445158600, 34315056105966195, 18412956934908690, 9206478467454345, 4282083008118300, 1849081298960175, 739632519584070, 273342452889765, 93052749919920, 29078984349975, 8308281242850, 2160153123141, 508271323092, 107518933731, 20286591270, 3381098545, 491796152, 61474519, 6471002, 557845, 37820, 1891, 62, 1, 0, 0],
    &[1, 63, 1953, 39711, 595665, 7028847, 67945521, 553270671, 3872894697, 23667689815, 127805525001, 615790256823, 2668424446233, 10468434365991, 37387265592825, 122131734269895, 366395202809685, 1012974972473835, 2588713818544245, 6131164307078475, 13488561475572645, 27619435402363035, 52728013040874885, 93993414551124795, 156655690918541325, 244382877832924467, 357174975294274221, 489462003181042451, 629308289804197437, 759510004936100355, 860778005594247069, 916312070471295267, 916312070471295267, 860778005594247069, 759510004936100355, 629308289804197437, 489462003181042451, 357174975294274221, 244382877832924467, 156655690918541325, 93993414551124795, 52728013040874885, 27619435402363035, 13488561475572645, 6131164307078475, 2588713818544245, 1012974972473835, 366395202809685, 122131734269895, 37387265592825, 10468434365991, 2668424446233, 615790256823, 127805525001, 23667689815, 3872894697, 553270671, 67945521, 7028847, 595665, 39711, 1953, 63, 1, 0],
    &[1, 64, 2016, 41664, 635376, 7624512, 74974368, 621216192, 4426165368, 27540584512, 151473214816, 743595781824, 3284214703056, 13136858812224, 47855699958816, 159518999862720, 488526937079580, 1379370175283520, 3601688791018080, 8719878125622720, 19619725782651120, 41107996877935680, 80347448443237920, 146721427591999680, 250649105469666120, 401038568751465792, 601557853127198688, 846636978475316672, 1118770292985239888, 1388818294740297792, 1620288010530347424, 1777090076065542336, 1832624140942590534, 1777090076065542336, 1620288010530347424, 1388818294740297792, 1118770292985239888, 846636978475316672, 601557853127198688, 401038568751465792, 250649105469666120, 146721427591999680, 80347448443237920, 41107996877935680, 19619725782651120, 8719878125622720, 3601688791018080, 1379370175283520, 488526937079580, 159518999862720, 47855699958816, 13136858812224, 3284214703056, 743595781824, 151473214816, 27540584512, 4426165368, 621216192, 74974368, 7624512, 635376, 41664, 2016, 64, 1],
];

// Precomputed number of bits it takes to represent a block of 64 bits where k
// of them are set: ceil(log(binomial_coefficient(64, k))) for k in [0, 64].
// However, note that once the code length is sufficiently long (> 46), we just
// set it to 64 to hit the fast paths above.  It's not worth using a variable
// length code to save a few bits when it makes computing `rank` and `select`
// much more expensive.
pub const ENUM_CODE_LENGTH: &[u8; 65] = &[
    0,   6, 11, 16, 20, 23, 27, 30, 33, 35, 38, 40, 42, 44, 46, 64,
    64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64,
    64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64, 64,
    64, 64, 46, 44, 42, 40, 38, 35, 33, 30, 27, 23, 20, 16, 11, 6,
    0
];

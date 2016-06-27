//! Broadword operations treating `u64` as a parallel vector.
//!
//! From [Sebastiano Vigna, “Broadword Implementation of
//! Rank/Select Queries”](http://sux.di.unimi.it/paper.pdf).

/// Has the lowest bit of every octet set: `0x0101010101010101`.
pub const L8: u64 = 0x0101010101010101;

/// Has the highest bit of every octet set: `0x8080808080808080`.
pub const H8: u64 = 0x8080808080808080;

/// Counts the number of ones in a `u64`.
///
/// Uses the broadword algorithm from Vigna.
pub fn count_ones(mut x: u64) -> usize {
    x = x.wrapping_sub((x & 0xAAAAAAAAAAAAAAAA) >> 1);
    x = (x & 0x3333333333333333).wrapping_add((x >> 2) & 0x3333333333333333);
    x = x.wrapping_add(x >> 4) & 0x0F0F0F0F0F0F0F0F;
    (x.wrapping_mul(L8) >> 56) as usize
}

/// Parallel ≤, treating a `u64` as a vector of 8 `u8`s.
pub fn u_le8(x: u64, y: u64) -> u64 {
    ((((y | H8).wrapping_sub(x & !H8)) | (x ^ y)) ^ (x & !y)) & H8
}

/// Parallel ≤, treating a `u64` as a vector of 8 `i8`s.
pub fn le8(x: u64, y: u64) -> u64 {
    (((y | H8).wrapping_sub(x & !H8)) ^ x ^ y) & H8
}

/// Parallel >0, treating a `u64` as a vector of 8 `u8`s.
pub fn u_nz8(x: u64) -> u64 {
    ((x | H8).wrapping_sub(L8) | x) & H8
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::quickcheck;

    #[test]
    fn count_ones_1() {
        assert_eq!(1, count_ones(1));
    }

    fn count_ones_prop(word: u64) -> bool {
        count_ones(word) == word.count_ones() as usize
    }

    #[test]
    fn count_ones_works() {
        quickcheck(count_ones_prop as fn(u64) -> bool);
    }

    #[test]
    fn u_gt_0_works() {
        assert_eq!(b(0, 0, 0, 0, 0, 0, 0, 0),
            u_nz8(u(0, 0, 0, 0, 0, 0, 0, 0)));

        assert_eq!(b( 1,  1, 0,   1, 0, 1,  1, 1),
            u_nz8(u(45, 12, 0, 129, 0, 3, 80, 1)));

        assert_eq!(b(1, 1, 1, 1, 1, 1, 1, 1),
            u_nz8(u(1, 2, 3, 4, 5, 6, 7, 8)));

        assert_eq!(b( 1, 1, 1, 1, 0, 1, 1, 1),
            u_nz8(0xFF_FF_FF_FF_00_FF_FF_FF));
    }

    #[test]
    fn le8_works() {
        assert_eq!(b( 1,  1,  1,  1,  0,  0,  0,  0),
               le8(i( 0,  0,  0,  0,  0,  0,  0,  0),
                   i( 3,  2,  1,  0, -1, -2, -3, -4)));

        assert_eq!(b( 0,  0,  0,  1,  1,  1,  1,  1),
               le8(i( 3,  2,  1,  0, -1, -2, -3, -4),
                   i( 0,  0,  0,  0,  0,  0,  0,  0)));

        assert_eq!(b( 0,  0,  1,  1,  1,  1,  1,  1),
               le8(i(19, 18, 17, 16, 15,  0, -1, -2),
                   i(17, 17, 17, 17, 17, 17, 17, 17)));

        assert_eq!(b( 1,  1,  0,  0,  0,  0,  0,  0),
               le8(i(-9, -8, -7,  0,  1,  2,  3,  4),
                   i(-8, -8, -8, -8, -8, -8, -8, -8)));

        assert_eq!(b( 0,  1,  0,  1,  1,  0,  1,  0),
               le8(i( 8,  3, 46,  0,  0,  0, -6, -1),
                   i( 7,  3, 24,  1,  0, -9,  5, -2)));
    }

    #[test]
    fn u_le8_works() {
        assert_eq!(b( 1,  1,  1,  1,  1,  1,  1,  1),
             u_le8(u( 0,  0,  0,  0,  0,  0,  0,  0),
                   u( 7,  6,  5,  4,  3,  2,  1,  0)));

        assert_eq!(b( 1,  0,  0,  0,  0,  0,  0,  0),
             u_le8(u( 0,  1,  2,  3,  4,  5,  6,  7),
                   u( 0,  0,  0,  0,  0,  0,  0,  0)));

        assert_eq!(b( 0,  0,  1,  1,  1,  1,  1,  1),
             u_le8(u(19, 18, 17, 16, 15, 14, 13, 12),
                   u(17, 17, 17, 17, 17, 17, 17, 17)));

        assert_eq!(b( 0,  1,  0,  1,  1,  0,  1,  0),
             u_le8(u( 8,  3, 46,  0,  0,  9,  3,  2),
                   u( 7,  3, 24,  1,  0,  0,  5,  1)));
    }

    /// Helpers for creating u64s.

    fn b(a: u64, b: u64, c: u64, d: u64,
         e: u64, f: u64, g: u64, h: u64) -> u64 {
        (a << 63) | (b << 55) | (c << 47) | (d << 39) |
                (e << 31) | (f << 23) | (g << 15) | (h << 7)
    }

    fn u(a: u8, b: u8, c: u8, d: u8,
         e: u8, f: u8, g: u8, h: u8) -> u64 {
        ((a as u64) << 56)
                | ((b as u64) << 48)
                | ((c as u64) << 40)
                | ((d as u64) << 32)
                | ((e as u64) << 24)
                | ((f as u64) << 16)
                | ((g as u64) << 8)
                | (h as u64)
    }

    fn i(a: i8, b: i8, c: i8, d: i8,
         e: i8, f: i8, g: i8, h: i8) -> u64 {
        u(a as u8, b as u8, c as u8, d as u8,
          e as u8, f as u8, g as u8, h as u8)
    }

}


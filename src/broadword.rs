//! Broadword operations treating `u64` as a parallel vector.
//!
//! From [Sebastiano Vigna, “Broadword Implementation of
//! Rank/Select Queries.”](http://sux.di.unimi.it/paper.pdf) Changes from
//! that work:
//!
//!   - It uses a 17-digit (68-bit) constant “0x0F0F0F0F0F0F0F0F0.” I believe
//!     the correct constant is these 64 bits: 0x0F0F_0F0F_0F0F_0F0F.
//!
//!   - Arithmetic operations are assumed to wrap on overflow. If this
//!     were not the case, Algorithm 1 ([count_ones](fn.count_ones.html))
//!     would overflow its last line, when multiplying by L₈.
//!
//!   - Line 2 of Algorithm 2 should read
//!
//!     ```
//!     # let mut s = 0u64;
//!     s = (s & 0x3333_3333_3333_3333) + ((s >> 2) & 0x3333_3333_3333_3333);
//!     ```
//!
//!     In the paper, the shifted `s` appears as `x`.

use rank::{BitRankSupport, RankSupport};
use select::Select1Support;
use storage::BlockType;

/// Newtype for treating a `u64` as a rank or select structure.
pub struct Broadword(pub u64);

impl BitRankSupport for Broadword {
    fn rank1(&self, position: u64) -> u64 {
        debug_assert!(position < 64);
        count_ones(self.0 & u64::low_mask(position as usize + 1)) as u64
    }
}

impl RankSupport for Broadword {
    type Over = bool;

    fn rank(&self, position: u64, value: bool) -> u64 {
        if value {
            self.rank1(position)
        } else {
            self.rank0(position)
        }
    }

    fn limit(&self) -> u64 {
        64
    }
}

impl Select1Support for Broadword {
    fn select1(&self, index: u64) -> Option<u64> {
        select1(index as usize, self.0).map(|u| u as u64)
    }
}

/// Has the lowest bit of every byte set: `0x0101_0101_0101_0101`.
pub const L8: u64 = 0x0101_0101_0101_0101;

/// Has the highest bit of every byte set: `0x8080_8080_8080_8080`.
pub const H8: u64 = 0x8080_8080_8080_8080;

/// Counts the number of ones in a `u64`.
///
/// Uses the broadword algorithm from Vigna.
pub fn count_ones(mut x: u64) -> usize {
    x = x - ((x & 0xAAAA_AAAA_AAAA_AAAA) >> 1);
    x = (x & 0x3333_3333_3333_3333) + ((x >> 2) & 0x3333_3333_3333_3333);
    x = (x + (x >> 4)) & 0x0F0F_0F0F_0F0F_0F0F;
    (x.wrapping_mul(L8) >> 56) as usize
}

/// Finds the index of the `r`th one bit in `x`.
///
/// Uses the broadword algorithm from Vigna.
pub fn select1(r: usize, x: u64) -> Option<usize> {
    let result = select1_raw(r, x);
    if result == 72 {
        None
    } else {
        Some(result)
    }
}

/// Finds the index of the `r`th one bit in `x`, returning 72 when not found.
///
/// Uses the broadword algorithm from Vigna.
pub fn select1_raw(r: usize, x: u64) -> usize {
    let r = r as u64;
    let mut s = x - ((x & 0xAAAA_AAAA_AAAA_AAAA) >> 1);
    s = (s & 0x3333_3333_3333_3333) + ((s >> 2) & 0x3333_3333_3333_3333);
    s = ((s + (s >> 4)) & 0x0F0F_0F0F_0F0F_0F0F).wrapping_mul(L8);
    let b = (le8(s, r.wrapping_mul(L8)) >> 7).wrapping_mul(L8) >> 53 & !7;
    let l = r - ((s << 8).wrapping_shr(b as u32) & 0xFF);
    s = (u_nz8((x.wrapping_shr(b as u32) & 0xFF).wrapping_mul(L8) & 0x8040_2010_0804_0201) >> 7)
        .wrapping_mul(L8);
    (b + ((le8(s, l.wrapping_mul(L8)) >> 7).wrapping_mul(L8) >> 56)) as usize
}

/// Parallel ≤, treating a `u64` as a vector of 8 `u8`s.
pub fn u_le8(x: u64, y: u64) -> u64 {
    ((((y | H8) - (x & !H8)) | (x ^ y)) ^ (x & !y)) & H8
}

/// Parallel ≤, treating a `u64` as a vector of 8 `i8`s.
pub fn le8(x: u64, y: u64) -> u64 {
    (((y | H8) - (x & !H8)) ^ x ^ y) & H8
}

/// Parallel >0, treating a `u64` as a vector of 8 `u8`s.
pub fn u_nz8(x: u64) -> u64 {
    (((x | H8) - L8) | x) & H8
}

#[cfg(test)]
mod test {
    use quickcheck::{quickcheck, TestResult};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use super::*;
    use select::{BinSearchSelect, Select1Support};

    #[test]
    fn count_ones_0() {
        assert_eq!(0, count_ones(0));
    }

    #[test]
    fn count_ones_1() {
        assert_eq!(1, count_ones(1));
    }

    #[test]
    fn count_ones_0000_0000_0000_0010() {
        assert_eq!(1, count_ones(0x0000_0000_0000_0010));
    }

    #[test]
    fn count_ones_1000_0000_0000_0000() {
        assert_eq!(1, count_ones(0x1000_0000_0000_0000));
    }

    #[test]
    fn count_ones_ffff_ffff_ffff_ffff() {
        assert_eq!(64, count_ones(0xFFFF_FFFF_FFFF_FFFF));
    }

    fn count_ones_prop(word: u64) -> bool {
        count_ones(word) == word.count_ones() as usize
    }

    fn count_ones_prop_hash(word: u64) -> bool {
        count_ones_prop(hash(&word))
    }

    #[test]
    fn count_ones_qc() {
        quickcheck(count_ones_prop as fn(u64) -> bool);
    }

    #[test]
    fn count_ones_qc_hash() {
        quickcheck(count_ones_prop_hash as fn(u64) -> bool);
    }

    #[test]
    fn select1_0_0() {
        assert_eq!(None, select1(0, 0));
    }

    #[test]
    fn select1_0_1() {
        assert_eq!(Some(0), select1(0, 1));
    }

    #[test]
    fn select1_0_2() {
        assert_eq!(Some(1), select1(0, 2));
    }

    #[test]
    fn select1_0_3() {
        assert_eq!(Some(0), select1(0, 3));
    }

    #[test]
    fn select1_1_2() {
        assert_eq!(None, select1(1, 2));
    }

    #[test]
    fn select1_1_3() {
        assert_eq!(Some(1), select1(1, 3));
    }

    #[test]
    fn select1_3_13() {
        assert_eq!(None, select1(3, 0b1101));
    }

    fn select1_prop(r: u8, x: u64) -> TestResult {
        if r > 64 {
            return TestResult::discard();
        }

        let ss = BinSearchSelect::new(x);
        TestResult::from_bool(select1(r as usize, x).map(|n| n as u64) == ss.select1(r as u64))
    }

    fn select1_prop_hash(r: u8, x: u64) -> TestResult {
        select1_prop(r, hash(&x))
    }

    #[test]
    fn select1_qc() {
        quickcheck(select1_prop as fn(u8, u64) -> TestResult);
    }

    #[test]
    fn select1_qc_hash() {
        quickcheck(select1_prop_hash as fn(u8, u64) -> TestResult);
    }

    fn u_nz8_prop((n0, n1, n2, n3): (u64, u64, u64, u64)) -> bool {
        let n = hash(&(n0, n1, n2, n3));
        let r = u_nz8(n);
        for i in 0..8 {
            let ni = n.get_bits(8 * i, 8);
            let ri = r.get_bits(8 * i, 8);
            if (ni != 0) != (ri == 0x80) {
                return false;
            }
        }

        true
    }

    #[test]
    fn u_nz8_qc() {
        quickcheck(u_nz8_prop as fn((u64, u64, u64, u64)) -> bool);
    }

    #[test]
    fn u_nz8_works() {
        assert_eq!(b(0, 0, 0, 0, 0, 0, 0, 0), u_nz8(u(0, 0, 0, 0, 0, 0, 0, 0)));

        assert_eq!(
            b(1, 1, 0, 1, 0, 1, 1, 1),
            u_nz8(u(45, 12, 0, 129, 0, 3, 80, 1))
        );

        assert_eq!(b(1, 1, 1, 1, 1, 1, 1, 1), u_nz8(u(1, 2, 3, 4, 5, 6, 7, 8)));

        assert_eq!(b(1, 1, 1, 1, 0, 1, 1, 1), u_nz8(0xFF_FF_FF_FF_00_FF_FF_FF));
    }

    //    // I don’t understand le8, apparently.

    //    #[test]
    //    fn le8_128_0() {
    //        let n = 128;
    //        let m = 0;
    //        let r = le8(n, m);
    //        let n0 = n.get_bits(0, 8) as u8 as i8;
    //        let m0 = m.get_bits(0, 8) as u8 as i8;
    //        let r0 = r.get_bits(0, 8);
    //        println!("n0: {}, m0: {}, r0: {}", n0, m0, r0);
    //        assert_eq!(n0 <= m0, r0 == 0x80);
    //    }

    //    fn le8_prop_hashed((n0, n1, n2, n3): (u64, u64, u64, u64),
    //                       (m0, m1, m2, m3): (u64, u64, u64, u64)) -> bool {
    //        let n = hash(&(n0, n1, n2, n3));
    //        let m = hash(&(m0, m1, m2, m3));
    //        le8_prop(n, m)
    //    }
    //
    //    fn le8_prop(n: u64, m: u64) -> bool {
    //        let r = le8(n, m);
    //        for i in 0..8 {
    //            let ni = n.get_bits(8 * i, 8) as u8 as i8;
    //            let mi = m.get_bits(8 * i, 8) as u8 as i8;
    //            let ri = r.get_bits(8 * i, 8);
    //            if (ni <= mi) != (ri == 0x80) {
    //                return false;
    //            }
    //        }
    //
    //        true
    //    }
    //
    //    #[test]
    //    fn le8_qc() {
    //        quickcheck(le8_prop as fn(u64, u64) -> bool);
    //    }
    //
    //    #[test]
    //    fn le8_qc_hashed() {
    //        quickcheck(le8_prop_hashed as fn((u64, u64, u64, u64),
    //                                         (u64, u64, u64, u64)) -> bool);
    //    }

    fn u_le8_prop_hashed(
        (n0, n1, n2, n3): (u64, u64, u64, u64),
        (m0, m1, m2, m3): (u64, u64, u64, u64),
    ) -> bool {
        let n = hash(&(n0, n1, n2, n3));
        let m = hash(&(m0, m1, m2, m3));
        u_le8_prop(n, m)
    }

    fn u_le8_prop(n: u64, m: u64) -> bool {
        let r = u_le8(n, m);
        for i in 0..8 {
            let ni = n.get_bits(8 * i, 8);
            let mi = m.get_bits(8 * i, 8);
            let ri = r.get_bits(8 * i, 8);
            if (ni <= mi) != (ri == 0x80) {
                return false;
            }
        }

        true
    }

    #[test]
    fn u_le8_qc() {
        quickcheck(u_le8_prop as fn(u64, u64) -> bool);
    }

    #[test]
    fn u_le8_qc_hashed() {
        quickcheck(u_le8_prop_hashed as fn((u64, u64, u64, u64), (u64, u64, u64, u64)) -> bool);
    }

    #[test]
    fn le8_works() {
        assert_eq!(
            b(1, 1, 1, 1, 0, 0, 0, 0),
            le8(i(0, 0, 0, 0, 0, 0, 0, 0), i(3, 2, 1, 0, -1, -2, -3, -4))
        );

        assert_eq!(
            b(0, 0, 0, 1, 1, 1, 1, 1),
            le8(i(3, 2, 1, 0, -1, -2, -3, -4), i(0, 0, 0, 0, 0, 0, 0, 0))
        );

        assert_eq!(
            b(0, 0, 1, 1, 1, 1, 1, 1),
            le8(
                i(19, 18, 17, 16, 15, 0, -1, -2),
                i(17, 17, 17, 17, 17, 17, 17, 17)
            )
        );

        assert_eq!(
            b(1, 1, 0, 0, 0, 0, 0, 0),
            le8(
                i(-9, -8, -7, 0, 1, 2, 3, 4),
                i(-8, -8, -8, -8, -8, -8, -8, -8)
            )
        );

        assert_eq!(
            b(0, 1, 0, 1, 1, 0, 1, 0),
            le8(i(8, 3, 46, 0, 0, 0, -6, -1), i(7, 3, 24, 1, 0, -9, 5, -2))
        );
    }

    #[test]
    fn u_le8_works() {
        assert_eq!(
            b(1, 1, 1, 1, 1, 1, 1, 1),
            u_le8(u(0, 0, 0, 0, 0, 0, 0, 0), u(7, 6, 5, 4, 3, 2, 1, 0))
        );

        assert_eq!(
            b(1, 0, 0, 0, 0, 0, 0, 0),
            u_le8(u(0, 1, 2, 3, 4, 5, 6, 7), u(0, 0, 0, 0, 0, 0, 0, 0))
        );

        assert_eq!(
            b(0, 0, 1, 1, 1, 1, 1, 1),
            u_le8(
                u(19, 18, 17, 16, 15, 14, 13, 12),
                u(17, 17, 17, 17, 17, 17, 17, 17)
            )
        );

        assert_eq!(
            b(0, 1, 0, 1, 1, 0, 1, 0),
            u_le8(u(8, 3, 46, 0, 0, 9, 3, 2), u(7, 3, 24, 1, 0, 0, 5, 1))
        );
    }

    /// Helpers for creating u64s.

    fn b(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64, g: u64, h: u64) -> u64 {
        (a << 63) | (b << 55) | (c << 47) | (d << 39) | (e << 31) | (f << 23) | (g << 15) | (h << 7)
    }

    fn u(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) -> u64 {
        ((a as u64) << 56)
            | ((b as u64) << 48)
            | ((c as u64) << 40)
            | ((d as u64) << 32)
            | ((e as u64) << 24)
            | ((f as u64) << 16)
            | ((g as u64) << 8)
            | (h as u64)
    }

    fn i(a: i8, b: i8, c: i8, d: i8, e: i8, f: i8, g: i8, h: i8) -> u64 {
        u(
            a as u8, b as u8, c as u8, d as u8, e as u8, f as u8, g as u8, h as u8,
        )
    }

    fn hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }
}

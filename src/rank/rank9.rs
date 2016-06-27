use num::ToPrimitive;

use bit_vector::Bits;
use rank::{RankSupport, BitRankSupport};
use storage::BlockType;

/// Rank support structure from Sebastiano Vigna.
#[derive(Clone, Debug)]
pub struct Rank9<Store: Bits> {
    bit_store: Store,
    counts: Vec<Rank9Cell>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Rank9Cell {
    level1: u64,
    level2: Level2,
}

#[derive(Clone, Copy, Debug)]
struct Level2(u64);

impl Level2 {
    fn new() -> Self { Level2(0) }

    fn get(&self, t: usize) -> u64 {
        debug_assert!(t < 8);

        let t = t.wrapping_sub(1);
        let shift = t.wrapping_add(t >> 60 & 8) * 9;
        self.0 >> shift & 0x1FF
    }

    fn set(&mut self, t: usize, value: u64) {
        debug_assert!(t < 8);

        let t = t.wrapping_sub(1);
        let shift = t.wrapping_add(t >> 60 & 8) * 9;

        let old_part = self.0 & !(0x1FF << shift);
        let new_part = (value & 0x1FF) << shift;

        self.0 = old_part | new_part;
    }
}

impl<Store: Bits> Rank9<Store> {
    pub fn new(bits: Store) -> Self {
        let basic_block_bits = 8 * 64;
        let basic_block_count = bits.bit_len()
                                    .ceil_div(basic_block_bits)
                                    .to_usize()
                                    .expect("Rank9::new: overflow");
        let result = Vec::with_capacity(basic_block_count);

        Rank9 {
            bit_store: bits,
            counts: result,
        }
    }
}

#[test]
fn level2() {
    let mut l2 =
        Level2(0b0_110010000_000000000_000000001_000001110_000001000_100000000_000000101);

    assert_eq!(0, l2.get(0));
    assert_eq!(5, l2.get(1));
    assert_eq!(256, l2.get(2));
    assert_eq!(8, l2.get(3));
    assert_eq!(14, l2.get(4));
    assert_eq!(1, l2.get(5));
    assert_eq!(0, l2.get(6));
    assert_eq!(400, l2.get(7));

    l2.set(3, 45);

    assert_eq!(0, l2.get(0));
    assert_eq!(5, l2.get(1));
    assert_eq!(256, l2.get(2));
    assert_eq!(45, l2.get(3));
    assert_eq!(14, l2.get(4));
    assert_eq!(1, l2.get(5));
    assert_eq!(0, l2.get(6));
    assert_eq!(400, l2.get(7));

    l2.set(7, 511);

    assert_eq!(0, l2.get(0));
    assert_eq!(5, l2.get(1));
    assert_eq!(256, l2.get(2));
    assert_eq!(45, l2.get(3));
    assert_eq!(14, l2.get(4));
    assert_eq!(1, l2.get(5));
    assert_eq!(0, l2.get(6));
    assert_eq!(511, l2.get(7));
}

use num::ToPrimitive;

use bit_vector::Bits;
use rank::{RankSupport, BitRankSupport};
use storage::BlockType;

/// Rank support structure from Sebastiano Vigna.
#[derive(Clone, Debug)]
pub struct Rank9<Store: Bits<Block = u64>> {
    bit_store: Store,
    counts: Vec<Rank9Cell>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Rank9Cell {
    level1: u64,
    level2: Level2,
}

#[repr(C)]
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

impl<Store: Bits<Block = u64>> Rank9<Store> {
    /// Creates a new rank9 structure.
    pub fn new(bits: Store) -> Self {
        let bb_count = bits.block_len().ceil_div(8);
        let mut result = Vec::with_capacity(bb_count + 1);

        let mut level1_count = 0;
        let mut level2_count = 0;

        // Scope for store_counts's borrow of result
        {
            let mut store_counts = |i: usize,
                                    level1_count: &mut u64,
                                    level2_count: &mut u64| {
                let basic_block_index = i / 8;
                let word_offset       = i % 8;

                if word_offset == 0 {
                    result.push(Rank9Cell {
                        level1: *level1_count,
                        level2: Level2::new(),
                    });
                    *level2_count = 0;
                } else {
                    result[basic_block_index].level2
                            .set(word_offset, *level2_count);
                }
            };

            for i in 0..bits.block_len() {
                store_counts(i, &mut level1_count, &mut level2_count);

                let word_count = bits.get_block(i).count_ones() as u64;
                level1_count += word_count;
                level2_count += word_count;
            }

            store_counts(bits.block_len(),
                         &mut level1_count, &mut level2_count);
        }

        Rank9 {
            bit_store: bits,
            counts: result,
        }
    }
}

impl<Store: Bits<Block = u64>> BitRankSupport for Rank9<Store> {
    fn rank1(&self, position: u64) -> u64 {
        let bb_index = (position / 512).to_usize()
                                       .expect("Rank9::rank1: index overflow");
        let word_index = (position / 64).to_usize()
                                        .expect("Rank9::rank1: index overflow");
        let word_offset = word_index % 8;
        let bit_offset = position % 64;

        let cell = self.counts[bb_index];

        let bb_portion = cell.level1;
        let word_portion = cell.level2.get(word_offset);
        let bit_portion = self.bit_store.get_block(word_index)
                                        .rank1(bit_offset);

        bb_portion + word_portion + bit_portion
    }
}

impl<Store: Bits<Block = u64>> RankSupport for Rank9<Store> {
    type Over = bool;

    fn rank(&self, position: u64, value: bool) -> u64 {
        if value {self.rank1(position)} else {self.rank0(position)}
    }

    fn limit(&self) -> u64 {
        self.bit_store.bit_len()
    }
}

impl_stack_only_space_usage!(Rank9Cell);
impl_stack_only_space_usage!(Level2);

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

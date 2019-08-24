use num_traits::ToPrimitive;

use bit_vec::BitVec;
use rank::{BitRankSupport, RankSupport};
use space_usage::SpaceUsage;
use storage::BlockType;

/// Vigna’s rank structure for fast rank queries over a `BitVec`.
#[derive(Clone, Debug)]
pub struct Rank9<Store> {
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
    fn new() -> Self {
        Level2(0)
    }

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

impl<Store: BitVec<Block = u64>> Rank9<Store> {
    /// Creates a new rank9 structure.
    pub fn new(bits: Store) -> Self {
        let bb_count = bits.block_len().ceil_div(8);
        let mut result = Vec::with_capacity(bb_count + 1);

        let mut level1_count = 0;
        let mut level2_count = 0;

        // Scope for store_counts's borrow of result
        {
            let mut store_counts = |i: usize, level1_count: &mut u64, level2_count: &mut u64| {
                let basic_block_index = i / 8;
                let word_offset = i % 8;

                if word_offset == 0 {
                    result.push(Rank9Cell {
                        level1: *level1_count,
                        level2: Level2::new(),
                    });
                    *level2_count = 0;
                } else {
                    result[basic_block_index]
                        .level2
                        .set(word_offset, *level2_count);
                }
            };

            for i in 0..bits.block_len() {
                store_counts(i, &mut level1_count, &mut level2_count);

                let word_count = bits.get_block(i).count_ones() as u64;
                level1_count += word_count;
                level2_count += word_count;
            }

            store_counts(bits.block_len(), &mut level1_count, &mut level2_count);
        }

        Rank9 {
            bit_store: bits,
            counts: result,
        }
    }

    /// Borrows a reference to the underlying bit store.
    pub fn inner(&self) -> &Store {
        &self.bit_store
    }

    /// Returns the underlying bit store.
    pub fn into_inner(self) -> Store {
        self.bit_store
    }
}

impl<Store: BitVec<Block = u64>> BitRankSupport for Rank9<Store> {
    fn rank1(&self, position: u64) -> u64 {
        let bb_index = (position / 512)
            .to_usize()
            .expect("Rank9::rank1: index overflow");
        let word_index = (position / 64)
            .to_usize()
            .expect("Rank9::rank1: index overflow");
        let word_offset = word_index % 8;
        let bit_offset = position % 64;

        let cell = self.counts[bb_index];

        let bb_portion = cell.level1;
        let word_portion = cell.level2.get(word_offset);
        let bit_portion = self.bit_store.get_block(word_index).rank1(bit_offset);

        bb_portion + word_portion + bit_portion
    }
}

impl<Store: BitVec<Block = u64>> RankSupport for Rank9<Store> {
    type Over = bool;

    fn rank(&self, position: u64, value: bool) -> u64 {
        if value {
            self.rank1(position)
        } else {
            self.rank0(position)
        }
    }

    fn limit(&self) -> u64 {
        self.bit_store.bit_len()
    }
}

impl<Store: BitVec<Block = u64>> BitVec for Rank9<Store> {
    impl_bit_vec_adapter!(u64, bit_store);
}

impl_stack_only_space_usage!(Rank9Cell);
impl_stack_only_space_usage!(Level2);

impl<Store: SpaceUsage> SpaceUsage for Rank9<Store> {
    fn is_stack_only() -> bool {
        false
    }

    fn heap_bytes(&self) -> usize {
        self.bit_store.heap_bytes() + self.counts.heap_bytes()
    }
}

#[test]
fn level2() {
    let mut l2 = Level2(0b0_110010000_000000000_000000001_000001110_000001000_100000000_000000101);

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

#[cfg(test)]
mod test {
    use super::*;
    use rank::BitRankSupport;

    #[test]
    fn rank1() {
        let vec = vec![0b00000000000001110000000000000001u64; 1024];
        let rank = Rank9::new(vec);

        assert_eq!(1, rank.rank1(0));
        assert_eq!(1, rank.rank1(1));
        assert_eq!(1, rank.rank1(2));
        assert_eq!(1, rank.rank1(7));
        assert_eq!(2, rank.rank1(16));
        assert_eq!(3, rank.rank1(17));
        assert_eq!(4, rank.rank1(18));
        assert_eq!(4, rank.rank1(19));
        assert_eq!(4, rank.rank1(20));

        assert_eq!(16, rank.rank1(4 * 64 - 1));
        assert_eq!(17, rank.rank1(4 * 64));
        assert_eq!(2048, rank.rank1(512 * 64 - 1));
        assert_eq!(2049, rank.rank1(512 * 64));

        assert_eq!(4096, rank.rank1(1024 * 64 - 1));
    }

    // This test is a sanity check that we aren’t taking up too much
    // space with the metadata.
    #[test]
    fn space() {
        use space_usage::*;

        for i in 0..50 {
            let vec = vec![0u64; 1000 + i];
            let vec_bytes = vec.total_bytes() as f64;
            let rank = Rank9::new(vec);

            assert!(rank.total_bytes() as f64 / vec_bytes < 1.3);
        }
    }
}

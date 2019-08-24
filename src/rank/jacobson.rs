use num_traits::PrimInt;

use bit_vec::BitVec;
use int_vec::{IntVec, IntVector};
use space_usage::SpaceUsage;
use storage::{Address, BlockType};

use super::{BitRankSupport, RankSupport};

/// Jacobson’s rank structure for fast rank queries over a `BitVec`.
///
/// Construct with `JacobsonRank::new`.
#[derive(Clone, Debug)]
pub struct JacobsonRank<Store> {
    bit_store: Store,
    large_block_size: usize,
    large_block_ranks: IntVector<u64>,
    small_block_ranks: IntVector<u64>,
}

impl<Store: BitVec> JacobsonRank<Store> {
    /// Creates a new rank support structure for the given bit vector.
    pub fn new(bits: Store) -> Self {
        let n = bits.bit_len();
        let lg_n = n.ceil_lg();
        let lg2_n = lg_n * lg_n;

        let small_block_size = Store::Block::nbits();
        let small_per_large = lg2_n.ceil_div(small_block_size);
        let large_block_size = small_block_size * small_per_large;
        let large_block_count = n / large_block_size as u64 + 1;
        let small_block_count = n / small_block_size as u64 + 1;

        let large_meta_size = (n + 1).ceil_lg();
        let small_meta_size = (large_block_size + 1).ceil_lg();

        let mut large_block_ranks = IntVector::with_capacity(large_meta_size, large_block_count);
        let mut small_block_ranks = IntVector::with_capacity(small_meta_size, small_block_count);

        let mut current_rank: u64 = 0;
        let mut last_large_rank: u64 = 0;
        let mut small_block_index: usize = 0;

        for i in 0..bits.block_len() {
            if small_block_index == 0 {
                large_block_ranks.push(current_rank);
                last_large_rank = current_rank;
            }

            let excess_rank = current_rank - last_large_rank;
            small_block_ranks.push(excess_rank);

            current_rank += bits.get_block(i).count_ones() as u64;
            small_block_index += 1;

            if small_block_index == small_per_large {
                small_block_index = 0;
            }
        }

        large_block_ranks.push(current_rank);
        let excess_rank = current_rank - last_large_rank;
        small_block_ranks.push(excess_rank);

        JacobsonRank {
            bit_store: bits,
            large_block_size: large_block_size,
            large_block_ranks: large_block_ranks,
            small_block_ranks: small_block_ranks,
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

impl<Store: BitVec> RankSupport for JacobsonRank<Store> {
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

impl<Store: BitVec> BitRankSupport for JacobsonRank<Store> {
    fn rank1(&self, position: u64) -> u64 {
        assert!(
            position < self.bit_len(),
            "JacobsonRank::rank1: out of bounds"
        );

        let large_block = position / self.large_block_size as u64;
        let address = Address::new::<Store::Block>(position);

        let large_rank = self.large_block_ranks.get(large_block);
        let small_rank = self.small_block_ranks.get(address.block_index as u64);
        let bits_rank = self
            .bit_store
            .get_block(address.block_index)
            .rank1(address.bit_offset as u64);

        large_rank + small_rank + bits_rank
    }
}

impl<Store: BitVec> BitVec for JacobsonRank<Store> {
    impl_bit_vec_adapter!(Store::Block, bit_store);
}

impl<Store: SpaceUsage> SpaceUsage for JacobsonRank<Store> {
    #[inline]
    fn is_stack_only() -> bool {
        false
    }

    fn heap_bytes(&self) -> usize {
        self.large_block_ranks.heap_bytes()
            + self.small_block_ranks.heap_bytes()
            + self.bit_store.heap_bytes()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rank::BitRankSupport;

    #[test]
    fn rank1() {
        let vec = vec![0b00000000000001110000000000000001u32; 1024];
        let rank = JacobsonRank::new(vec);

        assert_eq!(1, rank.rank1(0));
        assert_eq!(1, rank.rank1(1));
        assert_eq!(1, rank.rank1(2));
        assert_eq!(1, rank.rank1(7));
        assert_eq!(2, rank.rank1(16));
        assert_eq!(3, rank.rank1(17));
        assert_eq!(4, rank.rank1(18));
        assert_eq!(4, rank.rank1(19));
        assert_eq!(4, rank.rank1(20));

        assert_eq!(16, rank.rank1(4 * 32 - 1));
        assert_eq!(17, rank.rank1(4 * 32));
        assert_eq!(2048, rank.rank1(512 * 32 - 1));
        assert_eq!(2049, rank.rank1(512 * 32));

        assert_eq!(4096, rank.rank1(1024 * 32 - 1));
    }

    // This test is a sanity check that we aren’t taking up too much
    // space with the metadata.
    #[test]
    fn space() {
        use space_usage::*;

        for i in 0..50 {
            let vec = vec![0b10000000000000001110000000000000u32; 1000 + i];
            let rank = JacobsonRank::new(&*vec);

            assert!((rank.total_bytes() as f64 / vec.total_bytes() as f64) < 1.5);
        }
    }
}

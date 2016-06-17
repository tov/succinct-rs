use num::PrimInt;

use int_vec::{IntVec, IntVecBuilder};
use space_usage::SpaceUsage;
use storage::{BitStore, BlockType};

use super::RankSupport;

/// Add-on to `BitStore` to support fast rank queries.
///
/// Construct with `JacobsonRank::new`.
#[derive(Clone, Debug)]
pub struct JacobsonRank<'a, Store: ?Sized + BitStore + 'a> {
    bit_store: &'a Store,
    large_block_size: usize,
    large_block_ranks: IntVec<u64>,
    small_block_ranks: IntVec<u64>,
}

fn ceil_log2<Block: BlockType>(block: Block) -> usize {
    if block <= Block::one() { return 0; }

    Block::nbits() - (block - Block::one()).leading_zeros() as usize
}

#[inline]
fn ceil_div<Block: BlockType>(dividend: Block, divisor: Block) -> Block {
    (dividend + divisor - Block::one()) / divisor
}

impl<'a, Store: BitStore + ?Sized + 'a>
JacobsonRank<'a, Store> {
    /// Creates a new rank support structure for the given bit vector.
    pub fn new(bits: &'a Store) -> Self {
        let n = bits.bit_len();
        let lg_n = ceil_log2(n);
        let lg2_n = lg_n * lg_n;

        let small_block_size  = Store::Block::nbits();
        let small_per_large   = ceil_div(lg2_n, small_block_size);
        let large_block_size  = small_block_size * small_per_large;
        let large_block_count = n / large_block_size as u64 + 1;
        let small_block_count = n / small_block_size as u64 + 1;

        let large_meta_size   = ceil_log2(n + 1);
        let small_meta_size   = ceil_log2(large_block_size + 1);

        let mut large_block_ranks =
            IntVecBuilder::new(large_meta_size)
                .capacity(large_block_count).build();
        let mut small_block_ranks =
            IntVecBuilder::new(small_meta_size)
                .capacity(small_block_count).build();

        let mut current_rank: u64 = 0;
        let mut last_large_rank: u64 = 0;
        let mut small_block_index: usize = 0;

        for i in 0 .. bits.block_len() {
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
}

impl<'a, Store: ?Sized + BitStore + 'a>
BitStore for JacobsonRank<'a, Store> {
    type Block = Store::Block;

    fn block_len(&self) -> usize {
        self.bit_store.block_len()
    }

    fn bit_len(&self) -> u64 {
        self.bit_store.bit_len()
    }

    fn get_block(&self, index: usize) -> Self::Block {
        self.bit_store.get_block(index)
    }

    fn get_bit(&self, index: u64) -> bool {
        self.bit_store.get_bit(index)
    }
}

impl<'a, Store: ?Sized + BitStore + 'a>
RankSupport for JacobsonRank<'a, Store> {
    fn rank(&self, position: u64) -> u64 {
        // Rank for any position past the end is the rank of the
        // last position.
        let position = ::std::cmp::min(position, self.bit_len() - 1);

        let small_block_size = Store::Block::nbits() as u64;

        let large_block = position / self.large_block_size as u64;
        let small_block = position / small_block_size;
        let bit_offset  = position % small_block_size;

        let large_rank = self.large_block_ranks.get(large_block);
        let small_rank = self.small_block_ranks.get(small_block);
        let bits_rank  =
            self.bit_store.get_block(small_block as usize)
                .unsigned_shr((small_block_size - bit_offset - 1) as u32)
                .count_ones() as u64;

        large_rank + small_rank + bits_rank
    }
}

impl<'a, Store: ?Sized + BitStore + 'a>
SpaceUsage for JacobsonRank<'a, Store> {
    #[inline]
    fn is_stack_only() -> bool { false }

    fn heap_bytes(&self) -> usize {
        self.large_block_ranks.heap_bytes()
            + self.small_block_ranks.heap_bytes()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::RankSupport;

    #[test]
    fn rank() {
        let vec = vec![ 0b10000000000000001110000000000000u32; 1024 ];
        let rank = JacobsonRank::new(&*vec);

        assert_eq!(1, rank.rank(0));
        assert_eq!(1, rank.rank(1));
        assert_eq!(1, rank.rank(2));
        assert_eq!(1, rank.rank(7));
        assert_eq!(2, rank.rank(16));
        assert_eq!(3, rank.rank(17));
        assert_eq!(4, rank.rank(18));
        assert_eq!(4, rank.rank(19));
        assert_eq!(4, rank.rank(20));

        assert_eq!(16, rank.rank(4 * 32 - 1));
        assert_eq!(17, rank.rank(4 * 32));
        assert_eq!(2048, rank.rank(512 * 32 - 1));
        assert_eq!(2049, rank.rank(512 * 32));

        assert_eq!(4096, rank.rank(1000000));
    }
}

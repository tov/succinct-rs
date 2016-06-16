//! Data structure to support fast rank queries.

use std::marker::PhantomData;

use bit_vector::{BitVector, Rank};
use block_type::BlockType;
use int_vec::{IntVec, IntVecBuilder};

/// Add-on to `BitVector` to support fast rank queries.
///
/// Construct with `RankSupport::new`.
#[derive(Clone, Debug)]
pub struct RankSupport<'a, Block, BV: 'a>
    where Block: BlockType,
          BV: BitVector<Block>
{
    bit_store: &'a BV,
    large_block_size: usize,
    large_block_ranks: IntVec<u64>,
    small_block_ranks: IntVec<u64>,
    marker: PhantomData<Block>
}

fn ceil_log2<Block: BlockType>(block: Block) -> usize {
    if block <= Block::one() { return 0; }

    Block::nbits() - (block - Block::one()).leading_zeros() as usize
}

impl<'a, Block, BV: 'a> RankSupport<'a, Block, BV>
    where Block: BlockType, BV: BitVector<Block>
{
    /// Creates a new rank support structure for the given bit vector.
    pub fn new(bits: &'a BV) -> Self {
        let n = bits.bit_len();
        let lg_n = ceil_log2(n);
        let lg2_n = lg_n * lg_n;

        let small_block_size: usize = Block::nbits();
        let small_per_large = (lg2_n + small_block_size - 1) / small_block_size;
        let large_block_size = small_block_size * small_per_large;
        let large_block_count = n / large_block_size as u64;
        let small_block_count = large_block_size as u64 * large_block_count;

        let large_meta_size = lg_n;
        let small_meta_size = ceil_log2(large_block_size);

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

        RankSupport {
            bit_store: bits,
            large_block_size: large_block_size,
            large_block_ranks: large_block_ranks,
            small_block_ranks: small_block_ranks,
            marker: PhantomData,
        }
    }
}

impl<'a, Block, BV: 'a> Rank for RankSupport<'a, Block, BV>
    where Block: BlockType, BV: BitVector<Block>
{
    fn rank(&self, position: u64) -> u64 {
        let small_block_size = Block::nbits() as u64;

        let large_block = position / self.large_block_size as u64;
        let small_block = position / small_block_size;
        let bit_offset  = position % small_block_size;

        let large_rank = self.large_block_ranks.get(large_block);
        let small_rank = self.small_block_ranks.get(small_block);
        let bits_rank  =
            (self.bit_store.get_block(small_block as usize)
             >> (small_block_size - bit_offset) as usize).count_ones() as u64;

        large_rank + small_rank + bits_rank
    }
}

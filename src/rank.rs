//! Data structure to support fast rank queries.

use std::marker::PhantomData;

use bit_vector::BitVector;
use block_type::BlockType;
use int_vec::IntVec;

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
    small_block_size: usize,
    large_block_ranks: IntVec,
    small_block_ranks: IntVec,
    marker: PhantomData<Block>
}

impl<'a, Block, BV: 'a> RankSupport<'a, Block, BV>
    where Block: BlockType, BV: BitVector<Block>
{
    /// Creates a new rank support structure for the given bit vector.
    pub fn new(bits: &BV) -> Self {
        unimplemented!()
    }
}

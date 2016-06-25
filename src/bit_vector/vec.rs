use bit_vector::traits::*;
use storage::BlockType;

impl<Block: BlockType> Bits for Vec<Block> {
    type Block = Block;

    #[inline]
    fn bit_len(&self) -> u64 {
        self.len() as u64 * Block::nbits() as u64
    }

    #[inline]
    fn block_len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn get_block(&self, position: usize) -> Block {
        self[position]
    }
}

impl<Block: BlockType> BitsMut for Vec<Block> {
    #[inline]
    fn set_block(&mut self, position: usize, value: Block) {
        self[position] = value;
    }
}

use std::io::Result;

use storage::BlockType;

trait BlockRead {
    type Block: BlockType;
    fn read_block(&self) -> Result<Block>;
}

trait BlockWrite {
    type Block: BlockType;
    fn write_block(&self, block: Block) -> Result<()>;
}

trait BitStream {
    /// The underlying numeric type for the bit source or sink.
    type Block: BlockType;

    /// The position in the stream, from the beginning, in bits.
    fn pos(&self) -> u64;

    /// Align the position to the next whole-byte boundary. This may be
    /// done by skipping input or emitting 0s.
    fn align_byte(&mut self);

    /// Align the position to the next whole-block boundary. This may be
    /// done by skipping input or emitting 0s.
    fn align_block(&mut self) {
        while self.pos() & Block::nbits() != 0 {
            self.align_byte()
        }
    }
}

/// Allows reading bits from a source.
trait BitRead : BitStream {
    /// Reads a single bit from the source.
    fn read_bit(&mut self) -> Result<bool> {
        self.read_int(1) == Block::one()
    }

    /// Reads an unsigned integer of `nbits`.
    fn read_int(&mut self, nbits: usize) -> Result<Block>;
}

/// Allows writing bits to a sink.
trait BitWrite : BitStream {
    /// Writes a single bit to the sink.
    fn write_bit(&mut self, value bool) -> Result<()> {
        self.write_int(if value {Block::one()} else {Block::zero()})
    }

    /// Writes an unsigned integer of `nbits`.
    fn write_int(&mut self, nbits: usize, value: Block) -> Result<()>;

    /// Writes out any bits in the buffer, filling with 0s if necessary
    /// to align to a block boundary.
    fn flush(&mut self);
}

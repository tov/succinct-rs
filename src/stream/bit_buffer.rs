use std::io::{Error, ErrorKind, Result};

use storage::{BlockType};
use stream::{BitRead, BitWrite};
use bit_vector::*;

/// A bit buffer can be used to read bits from or write bits to an
/// underlying bit vector.
#[derive(Clone, Debug)]
pub struct BitBuffer<Block: BlockType = usize> {
    data: BitVec<Block>,
    pos: u64,
}

impl<Block: BlockType> BitBuffer<Block> {
    /// Creates a new, empty bit buffer.
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Creates a new, empty bit buffer with the given capacity (in
    /// bits) preallocated.
    pub fn with_capacity(capacity: u64) -> Self {
        BitBuffer {
            data: BitVec::with_capacity(capacity),
            pos: 0,
        }
    }

    /// Creates a new bit buffer for reading from a bit vector.
    pub fn from(input: BitVec<Block>) -> Self {
        BitBuffer {
            data: input,
            pos: 0,
        }
    }

    /// Creates a new bit buffer for appending to a bit vector.
    pub fn append(vec: BitVec<Block>) -> Self {
        let len = vec.bit_len();
        BitBuffer {
            data: vec,
            pos: len,
        }
    }

    /// Returns the bit vector underlying the bit buffer.
    #[inline]
    pub fn into_inner(self) -> BitVec<Block> {
        self.data
    }

    /// Gives access to the bit vector underlying the bit buffer.
    #[inline]
    pub fn inner(&self) -> &BitVec<Block> {
        &self.data
    }

    /// The position in the bit buffer where the next read or write will
    /// occur.
    #[inline]
    pub fn position(&self) -> u64 {
        self.pos
    }

    /// Moves the position for the next read or write.
    pub fn seek(&mut self, position: u64) -> Result<()> {
        if position <= self.data.bit_len() {
            self.pos = position;
            Ok(())
        } else {
            Err(Error::new(ErrorKind::NotFound,
                           "position out of bounds"))
        }
    }
}

impl<Block: BlockType> Bits for BitBuffer<Block> {
    type Block = Block;

    #[inline]
    fn block_len(&self) -> usize {
        self.data.block_len()
    }

    #[inline]
    fn bit_len(&self) -> u64 {
        self.data.bit_len()
    }

    #[inline]
    fn get_block(&self, position: usize) -> Self::Block {
        self.data.get_block(position)
    }
}

impl<Block: BlockType> BitsMut for BitBuffer<Block> {
    #[inline]
    fn set_block(&mut self, position: usize, value: Self::Block) {
        self.data.set_block(position, value);
    }
}

impl<Block: BlockType> BitRead for BitBuffer<Block> {
    fn read_bit(&mut self) -> Result<Option<bool>> {
        if self.pos < self.bit_len() {
            let result = self.get_bit(self.pos);
            self.pos += 1;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }
}

impl<Block: BlockType> BitWrite for BitBuffer<Block> {
    fn write_bit(&mut self, value: bool) -> Result<()> {
        while self.pos >= self.bit_len() {
            self.data.push_bit(false);
        }

        let pos = self.pos;
        self.set_bit(pos, value);
        self.pos = pos + 1;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bit_vector::*;
    use stream::{BitRead, BitWrite};

    #[test]
    fn reader() {
        let mut vec = BitVec::<usize>::new();
        vec.push_bit(false);
        vec.push_bit(true);
        vec.push_bit(false);
        vec.push_bit(false);
        vec.push_bit(true);

        let mut reader = BitBuffer::from(vec);

        assert_eq!(Some(false), reader.read_bit().unwrap());
        assert_eq!(Some(true), reader.read_bit().unwrap());
        assert_eq!(Some(false), reader.read_bit().unwrap());
        assert_eq!(Some(false), reader.read_bit().unwrap());
        assert_eq!(Some(true), reader.read_bit().unwrap());
        assert_eq!(None, reader.read_bit().unwrap());
    }

    #[test]
    fn writer() {
        let mut writer = BitBuffer::<usize>::new();

        writer.write_bit(true).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(true).unwrap();
        writer.write_bit(true).unwrap();

        let mut vec = writer.into_inner();

        assert_eq!(Some(true), vec.pop_bit());
        assert_eq!(Some(true), vec.pop_bit());
        assert_eq!(Some(false), vec.pop_bit());
        assert_eq!(Some(false), vec.pop_bit());
        assert_eq!(Some(true), vec.pop_bit());
        assert_eq!(None, vec.pop_bit());
    }
}

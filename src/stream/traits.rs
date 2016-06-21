use std::io::{Error, ErrorKind, Result};

use num::PrimInt;

/// Allows reading bits from a source.
pub trait BitRead {
    /// Reads a single bit from the source.
    ///
    /// End-of-file is indicated by `Ok(None)`.
    fn read_bit(&mut self) -> Result<Option<bool>>;

    /// Reads `nbits` bits as an integer, least-significant bit first.
    fn read_int<N: PrimInt>(&mut self, nbits: usize) -> Result<N> {
        let mut result = N::zero();

        for _ in 0 .. nbits {
            if let Some(bit) = try!(self.read_bit()) {
                if bit {
                    result = result | N::one();
                }
                result = result << 1;
            } else {
                return
                    Err(Error::new(ErrorKind::InvalidInput,
                                   "BitRead::read_int: more bits expected"));
            }
        }

        Ok(result)
    }
}

/// Allows writing bits to a sink.
pub trait BitWrite {
    /// Writes a single bit to the sink.
    fn write_bit(&mut self, value: bool) -> Result<()>;

    /// Writes the lower `nbits` of `value`, least-significant first.
    fn write_int<N: PrimInt>(&mut self, nbits: usize, mut value: N) -> Result<()> {
        for _ in 0 .. nbits {
            try!(self.write_bit(value & N::one() == N::one()));
            value = value >> 1;
        }

        Ok(())
    }
}

// These instances aren't particularly efficient, but they might be good
// for testing.

use std::collections::VecDeque;

impl BitRead for VecDeque<bool> {
    fn read_bit(&mut self) -> Result<Option<bool>> {
        Ok(self.pop_front())
    }
}

impl BitWrite for VecDeque<bool> {
    fn write_bit(&mut self, value: bool) -> Result<()> {
        self.push_back(value);
        Ok(())
    }
}

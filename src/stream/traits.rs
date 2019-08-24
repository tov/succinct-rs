use std::io::Result;

use internal::errors::*;

use num_traits::PrimInt;

/// Allows reading bits from a source.
pub trait BitRead {
    /// Reads a single bit from the source.
    ///
    /// End-of-file is indicated by `Ok(None)`.
    fn read_bit(&mut self) -> Result<Option<bool>>;

    /// Reads `nbits` bits as an integer, least-significant bit first.
    fn read_int<N: PrimInt>(&mut self, nbits: usize) -> Result<Option<N>> {
        let mut result = N::zero();
        let mut mask = N::one();
        let mut consumed = false;

        for _ in 0..nbits {
            if let Some(bit) = try!(self.read_bit()) {
                consumed = true;
                if bit {
                    result = result | mask;
                }
                mask = mask << 1;
            } else {
                if consumed {
                    return out_of_bits("BitRead::read_int");
                } else {
                    return Ok(None);
                }
            }
        }

        Ok(Some(result))
    }

    /// Reads `nbits` bits as an integer, most-significant bit first.
    fn read_int_be<N: PrimInt>(&mut self, nbits: usize) -> Result<Option<N>> {
        let mut result = N::zero();
        let mut consumed = false;

        for _ in 0..nbits {
            if let Some(bit) = try!(self.read_bit()) {
                consumed = true;
                result = result << 1;
                if bit {
                    result = result | N::one()
                }
            } else {
                if consumed {
                    return out_of_bits("BitRead::read_int");
                } else {
                    return Ok(None);
                }
            }
        }

        Ok(Some(result))
    }
}

/// Allows writing bits to a sink.
pub trait BitWrite {
    /// Writes a single bit to the sink.
    fn write_bit(&mut self, value: bool) -> Result<()>;

    /// Writes the lower `nbits` of `value`, least-significant first.
    fn write_int<N: PrimInt>(&mut self, nbits: usize, mut value: N) -> Result<()> {
        for _ in 0..nbits {
            try!(self.write_bit(value & N::one() != N::zero()));
            value = value >> 1;
        }

        Ok(())
    }

    /// Writes the lower `nbits` of `value`, most-significant first.
    fn write_int_be<N: PrimInt>(&mut self, nbits: usize, value: N) -> Result<()> {
        let mut mask = N::one() << nbits - 1;

        for _ in 0..nbits {
            try!(self.write_bit(value & mask != N::zero()));
            mask = mask >> 1;
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

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn read_bit() {
        let mut vd = VecDeque::new();
        vd.push_back(true);
        vd.push_back(true);
        vd.push_back(false);

        assert_eq!(Some(true), vd.read_bit().unwrap());
        assert_eq!(Some(true), vd.read_bit().unwrap());
        assert_eq!(Some(false), vd.read_bit().unwrap());
        assert_eq!(None, vd.read_bit().unwrap());
    }

    #[test]
    fn write_bit() {
        let mut vd = VecDeque::new();

        vd.write_bit(false).unwrap();
        vd.write_bit(true).unwrap();
        vd.write_bit(true).unwrap();

        assert_eq!(Some(false), vd.pop_front());
        assert_eq!(Some(true), vd.pop_front());
        assert_eq!(Some(true), vd.pop_front());
        assert_eq!(None, vd.pop_front());
    }

    #[test]
    fn read_int() {
        let mut vd = VecDeque::new();

        vd.write_bit(false).unwrap();
        assert_eq!(Some(Some(0)), vd.read_int(1).ok());

        vd.write_bit(true).unwrap();
        assert_eq!(Some(Some(1)), vd.read_int(1).ok());

        vd.write_bit(true).unwrap();
        vd.write_bit(false).unwrap();
        assert_eq!(Some(Some(1)), vd.read_int(2).ok());

        vd.write_bit(false).unwrap();
        vd.write_bit(true).unwrap();
        assert_eq!(Some(Some(2)), vd.read_int(2).ok());

        vd.write_bit(true).unwrap();
        vd.write_bit(true).unwrap();
        assert_eq!(Some(Some(3)), vd.read_int(2).ok());

        vd.write_bit(true).unwrap();
        vd.write_bit(true).unwrap();
        vd.write_bit(false).unwrap();
        vd.write_bit(false).unwrap();
        assert_eq!(Some(Some(3)), vd.read_int(4).ok());
    }

    #[test]
    fn read_int_be() {
        let mut vd = VecDeque::new();

        vd.write_bit(false).unwrap();
        assert_eq!(Some(Some(0)), vd.read_int_be(1).ok());

        vd.write_bit(true).unwrap();
        assert_eq!(Some(Some(1)), vd.read_int_be(1).ok());

        vd.write_bit(true).unwrap();
        vd.write_bit(false).unwrap();
        assert_eq!(Some(Some(2)), vd.read_int_be(2).ok());

        vd.write_bit(false).unwrap();
        vd.write_bit(true).unwrap();
        assert_eq!(Some(Some(1)), vd.read_int_be(2).ok());

        vd.write_bit(true).unwrap();
        vd.write_bit(true).unwrap();
        assert_eq!(Some(Some(3)), vd.read_int_be(2).ok());

        vd.write_bit(true).unwrap();
        vd.write_bit(true).unwrap();
        vd.write_bit(false).unwrap();
        vd.write_bit(false).unwrap();
        assert_eq!(Some(Some(12)), vd.read_int_be(4).ok());
    }

    #[test]
    fn write_int() {
        let mut vd = VecDeque::new();

        vd.write_int(5, 6).unwrap();
        vd.write_int(5, 7).unwrap();
        vd.write_int(5, 2).unwrap();
        vd.write_int(4, 3).unwrap();
        vd.write_int(4, 1).unwrap();
        vd.write_int(4, 0).unwrap();
        vd.write_int(4, 6).unwrap();

        assert_eq!(Some(Some(6)), vd.read_int(5).ok());
        assert_eq!(Some(Some(7)), vd.read_int(5).ok());
        assert_eq!(Some(Some(2)), vd.read_int(5).ok());
        assert_eq!(Some(Some(3)), vd.read_int(4).ok());
        assert_eq!(Some(Some(1)), vd.read_int(4).ok());
        assert_eq!(Some(Some(0)), vd.read_int(4).ok());
        assert_eq!(Some(Some(6)), vd.read_int(4).ok());
    }

    #[test]
    fn write_int_be() {
        let mut vd = VecDeque::new();

        vd.write_int_be(5, 6).unwrap();
        vd.write_int_be(5, 7).unwrap();
        vd.write_int_be(5, 2).unwrap();
        vd.write_int_be(4, 3).unwrap();
        vd.write_int_be(4, 1).unwrap();
        vd.write_int_be(4, 0).unwrap();
        vd.write_int_be(4, 6).unwrap();

        assert_eq!(Some(Some(6)), vd.read_int_be(5).ok());
        assert_eq!(Some(Some(7)), vd.read_int_be(5).ok());
        assert_eq!(Some(Some(2)), vd.read_int_be(5).ok());
        assert_eq!(Some(Some(3)), vd.read_int_be(4).ok());
        assert_eq!(Some(Some(1)), vd.read_int_be(4).ok());
        assert_eq!(Some(Some(0)), vd.read_int_be(4).ok());
        assert_eq!(Some(Some(6)), vd.read_int_be(4).ok());
    }
}

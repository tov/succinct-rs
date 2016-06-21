use std::io::{Error, ErrorKind};

use num::PrimInt;

use super::*;
use stream::*;

/// Encodes $n$ as $n$ zeroes followed by a one.
pub struct Unary;

impl UniversalCode for Unary {
    fn encode<W: BitWrite, N: PrimInt>(sink: &mut W, mut value: N) -> Result<()> {
        while value > N::zero() {
            try!(sink.write_bit(false));
            value = value - N::one();
        }

        try!(sink.write_bit(true));

        Ok(())
    }

    fn decode<R: BitRead, N: PrimInt>(source: &mut R) -> Result<Option<N>> {
        let mut result = N::zero();

        while let Some(bit) = try!(source.read_bit()) {
            if bit { return Ok(Some(result)); }
            result = result + N::one();
        }

        if result == N::zero() {
            Ok(None)
        } else {
            Err(Error::new(ErrorKind::InvalidInput, "unary decode: more bits expected"))
        }
    }
}

use std::io::{Error, ErrorKind};

use super::*;
use stream::*;

/// Encodes $n$ as $n$ zeroes followed by a one.
pub struct Unary;

impl UniversalCode for Unary {
    fn encode<W: BitWrite>(sink: &mut W, mut value: u64) -> Result<()> {
        while value > 0 {
            try!(sink.write_bit(false));
            value = value - 1;
        }

        try!(sink.write_bit(true));

        Ok(())
    }

    fn decode<R: BitRead>(source: &mut R) -> Result<Option<u64>> {
        let mut result = 0;

        while let Some(bit) = try!(source.read_bit()) {
            if bit { return Ok(Some(result)); }
            result = result + 1;
        }

        if result == 0 {
            Ok(None)
        } else {
            Err(Error::new(ErrorKind::InvalidInput, "unary decode: more bits expected"))
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::VecDeque;
    use coding::*;

    #[test]
    fn test234() {
        let mut dv = VecDeque::<bool>::new();

        Unary::encode(&mut dv, 2).unwrap();
        Unary::encode(&mut dv, 3).unwrap();
        Unary::encode(&mut dv, 4).unwrap();

        assert_eq!(Some(2), Unary::decode(&mut dv).unwrap());
        assert_eq!(Some(3), Unary::decode(&mut dv).unwrap());
        assert_eq!(Some(4), Unary::decode(&mut dv).unwrap());
        assert_eq!(None, Unary::decode(&mut dv).unwrap());
    }
}

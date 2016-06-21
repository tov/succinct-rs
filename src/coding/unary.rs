use super::*;
use errors::*;
use stream::*;

/// Encodes _n_ as _n_ zeroes followed by a one.
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
        let mut consumed = false;

        while let Some(bit) = try!(source.read_bit()) {
            if bit { return Ok(Some(result)); }
            // This can't overflow because it would require too many
            // unary digits to get there:
            result = result + 1;
            consumed = true;
        }

        if consumed {
            out_of_bits("Unary::decode")
        } else {
            Ok(None)
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

use std::marker::PhantomData;

use super::*;
use errors::*;
use stream::*;

/// An Elias code.
///
/// Elias codes do not handle 0.
///
/// An Elias code first encodes the size of the number using some other
/// code—this is the `Header` parameter.
pub struct Elias<Header: UniversalCode>(PhantomData<Header>);

/// An Elias gamma code encodes the header in unary.
pub type Gamma = Elias<Unary>;

/// An Elias delta code encodes the header using the Elias gamma code.
pub type Delta = Elias<Gamma>;

/// An Elias omega code iterates the Elias encoding.
pub struct Omega;

impl<Header: UniversalCode> UniversalCode for Elias<Header> {
    fn encode<W: BitWrite>(sink: &mut W, value: u64) -> Result<()> {
        assert!(value != 0, "Elias codes do not handle 0");

        let nbits = 64 - value.leading_zeros() - 1;
        try!(Header::encode(sink, nbits as u64));
        sink.write_int(nbits as usize, value)
    }

    fn decode<R: BitRead>(source: &mut R) -> Result<Option<u64>> {
        if let Some(nbits) = try!(Header::decode(source)) {
            let low_bits: u64 = try!(source.read_int(nbits as usize));
            Ok(Some(low_bits | (1 << nbits)))
        } else {
            Ok(None)
        }
    }
}

impl UniversalCode for Omega {
    fn encode<W: BitWrite>(sink: &mut W, mut value: u64) -> Result<()> {
        let mut stack = Vec::<bool>::new();
        stack.push(false);

        while value > 1 {
            let nbits = 64 - value.leading_zeros();

            for _ in 0 .. nbits {
                stack.push(value & 1 == 1);
                value >>= 1;
            }

            value = nbits as u64 - 1;
        }

        while let Some(bit) = stack.pop() {
            try!(sink.write_bit(bit));
        }

        Ok(())
    }

    fn decode<R: BitRead>(source: &mut R) -> Result<Option<u64>> {
        let mut result: u64 = 1;

        loop {
            if let Some(bit) = try!(source.read_bit()) {
                if !bit { return Ok(Some(result)); }

                let mut next: u64 = 0;
                for _ in 0 .. result {
                    next <<= 1;

                    match try!(source.read_bit()) {
                        Some(true) => { next |= 1; }
                        Some(false) => { }
                        None => { return out_of_bits("Omega::decode"); }
                    }
                }

                result = next | (1 << result as u32)
            } else if result == 1 {
                return Ok(None);
            } else {
                return out_of_bits("Omega::decode");
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::VecDeque;
    use coding::*;

    #[test]
    fn gamma() {
        let mut dv = VecDeque::<bool>::new();

        Gamma::encode(&mut dv, 2).unwrap();
        Gamma::encode(&mut dv, 3).unwrap();
        Gamma::encode(&mut dv, 4).unwrap();

        assert_eq!(Some(2), Gamma::decode(&mut dv).unwrap());
        assert_eq!(Some(3), Gamma::decode(&mut dv).unwrap());
        assert_eq!(Some(4), Gamma::decode(&mut dv).unwrap());
        assert_eq!(None::<u64>, Gamma::decode(&mut dv).unwrap());
    }

    #[test]
    fn delta() {
        let mut dv = VecDeque::<bool>::new();

        Delta::encode(&mut dv, 2).unwrap();
        Delta::encode(&mut dv, 3).unwrap();
        Delta::encode(&mut dv, 38932).unwrap();
        Delta::encode(&mut dv, 4).unwrap();

        assert_eq!(Some(2), Delta::decode(&mut dv).unwrap());
        assert_eq!(Some(3), Delta::decode(&mut dv).unwrap());
        assert_eq!(Some(38932), Delta::decode(&mut dv).unwrap());
        assert_eq!(Some(4), Delta::decode(&mut dv).unwrap());
        assert_eq!(None::<u64>, Delta::decode(&mut dv).unwrap());
    }

    #[test]
    fn omega() {
        let mut dv = VecDeque::<bool>::new();

        Omega::encode(&mut dv, 2).unwrap();
        Omega::encode(&mut dv, 3).unwrap();
        Omega::encode(&mut dv, 38932).unwrap();
        Omega::encode(&mut dv, 4).unwrap();

        assert_eq!(Some(2), Omega::decode(&mut dv).unwrap());
        assert_eq!(Some(3), Omega::decode(&mut dv).unwrap());
        assert_eq!(Some(38932), Omega::decode(&mut dv).unwrap());
        assert_eq!(Some(4), Omega::decode(&mut dv).unwrap());
        assert_eq!(None::<u64>, Omega::decode(&mut dv).unwrap());
    }
}

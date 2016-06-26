use std::marker::PhantomData;

use super::*;
use internal::errors::*;
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
pub type Delta = Elias<Lift0<Gamma>>;

/// An Elias omega code iterates the Elias encoding.
pub struct Omega;

const WORD_BITS: u32 = 64;

impl<Header: UniversalCode> UniversalCode for Elias<Header> {
    fn encode<W: BitWrite>(sink: &mut W, value: u64) -> Result<()> {
        assert!(value != 0, "Elias codes do not handle 0");

        let nbits: u32 = WORD_BITS - 1 - value.leading_zeros();
        try!(Header::encode(sink, nbits as u64));
        sink.write_int(nbits as usize, value)
    }

    fn decode<R: BitRead>(source: &mut R) -> Result<Option<u64>> {
        if let Some(nbits) = try!(Header::decode(source)) {
            if nbits > WORD_BITS as u64 - 1 {
                return too_many_bits("Elias::decode");
            }

            let low_bits: u64 = try!(source.read_int(nbits as usize));
            Ok(Some(low_bits | (1 << nbits)))
        } else {
            Ok(None)
        }
    }
}

impl UniversalCode for Omega {
    fn encode<W: BitWrite>(sink: &mut W, mut value: u64) -> Result<()> {
        let mut stack = Vec::<(usize, u64)>::new();

        while value > 1 {
            let nbits = WORD_BITS - value.leading_zeros();
            stack.push((nbits as usize, value));
            value = nbits as u64 - 1;
        }

        while let Some((nbits, value)) = stack.pop() {
            try!(sink.write_int_be(nbits, value));
        }
        try!(sink.write_bit(false));

        Ok(())
    }

    fn decode<R: BitRead>(source: &mut R) -> Result<Option<u64>> {
        let mut result: u64 = 1;

        loop {
            if let Some(bit) = try!(source.read_bit()) {
                if !bit { return Ok(Some(result)); }

                let next: u64 = try!(source.read_int_be(result as usize));
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
    use quickcheck::quickcheck;
    use coding::*;
    use coding::properties;

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

    #[test]
    fn qc_gamma() {
        quickcheck(properties::code_decode::<Gamma> as fn(Vec<u64>) -> bool);
    }

    #[test]
    fn qc_delta() {
        quickcheck(properties::code_decode::<Delta> as fn(Vec<u64>) -> bool);
    }

    #[test]
    fn qc_omega() {
        quickcheck(properties::code_decode::<Omega> as fn(Vec<u64>) -> bool);
    }
}

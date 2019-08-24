use super::*;
use internal::errors::*;
use stream::*;

/// An Elias code.
///
/// Elias codes do not handle 0.
///
/// An Elias code first encodes the size of the number using some other
/// code—this is the `Header` parameter.
pub struct Elias<Header: UniversalCode>(pub Header);

/// An Elias gamma code encodes the header in unary.
pub type Gamma = Elias<Unary>;

/// An instance of `Gamma`.
pub const GAMMA: Gamma = Elias(Unary);

/// An Elias delta code encodes the header using the Elias gamma code.
pub type Delta = Elias<Lift0<Gamma>>;

/// An instance of `Delta`.
pub const DELTA: Delta = Elias(Lift0(GAMMA));

/// An Elias omega code iterates the Elias encoding.
pub struct Omega;

const WORD_BITS: u32 = 64;

impl<Header: UniversalCode> UniversalCode for Elias<Header> {
    fn encode<W: BitWrite>(&self, sink: &mut W, value: u64) -> Result<()> {
        assert!(value != 0, "Elias codes do not handle 0");

        let nbits: u32 = WORD_BITS - 1 - value.leading_zeros();
        try!(self.0.encode(sink, nbits as u64));
        sink.write_int(nbits as usize, value)
    }

    fn decode<R: BitRead>(&self, source: &mut R) -> Result<Option<u64>> {
        if let Some(nbits) = try!(self.0.decode(source)) {
            if nbits > WORD_BITS as u64 - 1 {
                return too_many_bits("Elias::decode");
            }

            if let Some(low_bits) = try!(source.read_int::<u64>(nbits as usize)) {
                Ok(Some(low_bits | (1 << nbits)))
            } else {
                out_of_bits("Elias::decode")
            }
        } else {
            Ok(None)
        }
    }
}

impl UniversalCode for Omega {
    fn encode<W: BitWrite>(&self, sink: &mut W, mut value: u64) -> Result<()> {
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

    fn decode<R: BitRead>(&self, source: &mut R) -> Result<Option<u64>> {
        let mut result: u64 = 1;

        loop {
            if let Some(bit) = try!(source.read_bit()) {
                if !bit {
                    return Ok(Some(result));
                }

                if let Some(next) = try!(source.read_int_be::<u64>(result as usize)) {
                    result = next | (1 << result as u32)
                } else {
                    return out_of_bits("Omega::decode");
                }
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
    use coding::properties;
    use coding::*;
    use quickcheck::quickcheck;
    use std::collections::VecDeque;

    #[test]
    fn gamma() {
        let mut dv = VecDeque::<bool>::new();

        GAMMA.encode(&mut dv, 2).unwrap();
        GAMMA.encode(&mut dv, 3).unwrap();
        GAMMA.encode(&mut dv, 4).unwrap();

        assert_eq!(Some(2), GAMMA.decode(&mut dv).unwrap());
        assert_eq!(Some(3), GAMMA.decode(&mut dv).unwrap());
        assert_eq!(Some(4), GAMMA.decode(&mut dv).unwrap());
        assert_eq!(None::<u64>, GAMMA.decode(&mut dv).unwrap());
    }

    #[test]
    fn delta() {
        let mut dv = VecDeque::<bool>::new();

        DELTA.encode(&mut dv, 2).unwrap();
        DELTA.encode(&mut dv, 3).unwrap();
        DELTA.encode(&mut dv, 38932).unwrap();
        DELTA.encode(&mut dv, 4).unwrap();

        assert_eq!(Some(2), DELTA.decode(&mut dv).unwrap());
        assert_eq!(Some(3), DELTA.decode(&mut dv).unwrap());
        assert_eq!(Some(38932), DELTA.decode(&mut dv).unwrap());
        assert_eq!(Some(4), DELTA.decode(&mut dv).unwrap());
        assert_eq!(None::<u64>, DELTA.decode(&mut dv).unwrap());
    }

    #[test]
    fn omega() {
        let mut dv = VecDeque::<bool>::new();

        Omega.encode(&mut dv, 2).unwrap();
        Omega.encode(&mut dv, 3).unwrap();
        Omega.encode(&mut dv, 38932).unwrap();
        Omega.encode(&mut dv, 4).unwrap();

        assert_eq!(Some(2), Omega.decode(&mut dv).unwrap());
        assert_eq!(Some(3), Omega.decode(&mut dv).unwrap());
        assert_eq!(Some(38932), Omega.decode(&mut dv).unwrap());
        assert_eq!(Some(4), Omega.decode(&mut dv).unwrap());
        assert_eq!(None::<u64>, Omega.decode(&mut dv).unwrap());
    }

    #[test]
    fn qc_gamma() {
        fn prop_gamma(v: Vec<u64>) -> bool {
            properties::code_decode(&GAMMA, v)
        }

        quickcheck(prop_gamma as fn(Vec<u64>) -> bool);
    }

    #[test]
    fn qc_delta() {
        fn prop_delta(v: Vec<u64>) -> bool {
            properties::code_decode(&DELTA, v)
        }

        quickcheck(prop_delta as fn(Vec<u64>) -> bool);
    }

    #[test]
    fn qc_omega() {
        fn prop_omega(v: Vec<u64>) -> bool {
            properties::code_decode(&Omega, v)
        }

        quickcheck(prop_omega as fn(Vec<u64>) -> bool);
    }
}

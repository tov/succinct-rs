use super::*;
use internal::errors::*;
use stream::*;

/// `Comma(n)` encodes in base 2<sup>n</sup> - 1, using n bits per digit.
pub struct Comma(pub u8);

/// `Comma(2)` encodes in base 3.
pub const COMMA: Comma = Comma(2);

impl UniversalCode for Comma {
    fn encode<W: BitWrite>(&self, sink: &mut W, mut value: u64) -> Result<()> {
        let base = (1 << self.0) - 1;
        let mut stack: Vec<u64> = Vec::new();

        while value > 0 {
            stack.push(value % base);
            value /= base;
        }

        while let Some(digit) = stack.pop() {
            try!(sink.write_int(self.0 as usize, digit));
        }

        try!(sink.write_int(self.0 as usize, base));

        Ok(())
    }

    fn decode<R: BitRead>(&self, source: &mut R) -> Result<Option<u64>> {
        let base = (1 << self.0) - 1;
        let mut result = 0;
        let mut consumed = false;

        loop {
            if let Some(digit) = try!(source.read_int::<u64>(self.0 as usize)) {
                if digit == base {
                    return Ok(Some(result));
                }

                consumed = true;
                result = result * base + digit;
            } else if consumed {
                return out_of_bits("Comma::decode");
            } else {
                return Ok(None);
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
    fn enc234() {
        let mut dv = VecDeque::<bool>::new();

        COMMA.encode(&mut dv, 2).unwrap();
        COMMA.encode(&mut dv, 3).unwrap();
        COMMA.encode(&mut dv, 4).unwrap();

        assert_eq!(Some(2), COMMA.decode(&mut dv).unwrap());
        assert_eq!(Some(3), COMMA.decode(&mut dv).unwrap());
        assert_eq!(Some(4), COMMA.decode(&mut dv).unwrap());
        assert_eq!(None::<u64>, COMMA.decode(&mut dv).unwrap());
    }

    #[test]
    fn qc_comma2() {
        fn prop(v: Vec<u64>) -> bool {
            properties::code_decode(&Comma(2), v)
        }

        quickcheck(prop as fn(Vec<u64>) -> bool);
    }

    #[test]
    fn qc_comma3() {
        fn prop(v: Vec<u64>) -> bool {
            properties::code_decode(&Comma(3), v)
        }

        quickcheck(prop as fn(Vec<u64>) -> bool);
    }

    #[test]
    fn qc_comma4() {
        fn prop(v: Vec<u64>) -> bool {
            properties::code_decode(&Comma(4), v)
        }

        quickcheck(prop as fn(Vec<u64>) -> bool);
    }
}

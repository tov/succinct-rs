use super::*;
use errors::*;
use stream::*;

/// A Fibonacci code.
pub struct Fibonacci;

impl UniversalCode for Fibonacci {
    fn encode<W: BitWrite>(sink: &mut W, mut value: u64) -> Result<()> {
        assert!(value != 0, "Fibonacci codes cannot handle 0.");
        let mut fib_i_1 = 0;
        let mut fib_i   = 1;

        while fib_i <= value {
            let fib_i_2 = fib_i_1;
            fib_i_1 = fib_i;
            fib_i = fib_i_1 + fib_i_2;
        }

        // Now fib_i_1 is the largest Fibonacci number <= value

        let mut stack = vec![true];
        while fib_i > 1 {
            if fib_i_1 <= value {
                value -= fib_i_1;
                stack.push(true);
            } else {
                stack.push(false);
            }

            let fib_i_2 = fib_i - fib_i_1;
            fib_i = fib_i_1;
            fib_i_1 = fib_i_2;
        }

        while let Some(bit) = stack.pop() {
            try!(sink.write_bit(bit));
        }

        Ok(())
    }

    fn decode<R: BitRead>(source: &mut R) -> Result<Option<u64>> {
        unimplemented!();
    }
}

#[cfg(test)]
mod test {
    use std::collections::VecDeque;
    use quickcheck::quickcheck;
    use coding::*;
    use coding::properties;

    #[test]
    fn enc234() {
        let mut dv = VecDeque::<bool>::new();

        Fibonacci::encode(&mut dv, 2).unwrap();
        Fibonacci::encode(&mut dv, 3).unwrap();
        Fibonacci::encode(&mut dv, 4).unwrap();

        // assert_eq!(Some(2), Fibonacci::decode(&mut dv).unwrap());
        // assert_eq!(Some(3), Fibonacci::decode(&mut dv).unwrap());
        // assert_eq!(Some(4), Fibonacci::decode(&mut dv).unwrap());
        // assert_eq!(None::<u64>, Fibonacci::decode(&mut dv).unwrap());
    }

}

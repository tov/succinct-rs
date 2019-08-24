use std::mem;

use super::*;
use internal::errors::*;
use stream::*;

/// A Fibonacci code.
pub struct Fibonacci;

struct Fib {
    i_1: u64,
    i: u64,
}

impl Fib {
    fn new() -> Self {
        Fib { i_1: 1, i: 1 }
    }

    fn next(&mut self) -> Result<()> {
        if let Some(next) = self.i_1.checked_add(self.i) {
            self.i_1 = self.i;
            self.i = next;
            Ok(())
        } else {
            too_many_bits("Fibonacci")
        }
    }

    fn prev(&mut self) {
        self.i -= self.i_1;
        mem::swap(&mut self.i, &mut self.i_1);
    }
}

impl UniversalCode for Fibonacci {
    fn encode<W: BitWrite>(&self, sink: &mut W, mut value: u64) -> Result<()> {
        assert!(value != 0, "Fibonacci codes cannot handle 0.");
        let mut fib = Fib::new();

        // Having to compute fib.i when we really just need fib.i_1
        // means that this gives up on smaller numbers than it needs to.
        while fib.i <= value {
            try!(fib.next());
        }

        // Now fib.i_1 is the largest Fibonacci number <= value

        let mut stack = vec![true];
        while fib.i > 1 {
            if fib.i_1 <= value {
                value -= fib.i_1;
                stack.push(true);
            } else {
                stack.push(false);
            }

            fib.prev();
        }

        while let Some(bit) = stack.pop() {
            try!(sink.write_bit(bit));
        }

        Ok(())
    }

    fn decode<R: BitRead>(&self, source: &mut R) -> Result<Option<u64>> {
        let mut result = 0;
        let mut fib = Fib::new();
        let mut previous = false;

        while let Some(bit) = try!(source.read_bit()) {
            if bit && previous {
                return Ok(Some(result));
            }

            if bit {
                result += fib.i;
            }

            try!(fib.next());
            previous = bit;
        }

        if result == 0 {
            Ok(None)
        } else {
            out_of_bits("Fibonacci::decode")
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

        Fibonacci.encode(&mut dv, 2).unwrap();
        Fibonacci.encode(&mut dv, 3).unwrap();
        Fibonacci.encode(&mut dv, 4).unwrap();

        assert_eq!(Some(2), Fibonacci.decode(&mut dv).unwrap());
        assert_eq!(Some(3), Fibonacci.decode(&mut dv).unwrap());
        assert_eq!(Some(4), Fibonacci.decode(&mut dv).unwrap());
        assert_eq!(None::<u64>, Fibonacci.decode(&mut dv).unwrap());
    }

    #[test]
    fn qc() {
        fn prop(v: Vec<u64>) -> bool {
            properties::code_decode(&Fibonacci, v)
        }

        quickcheck(prop as fn(Vec<u64>) -> bool);
    }
}

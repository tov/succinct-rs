//! Codes used for data compression.

mod traits;
pub use self::traits::*;

mod unary;
pub use self::unary::*;

mod elias;
pub use self::elias::*;

mod fib;
pub use self::fib::*;

mod trans;
pub use self::trans::*;

#[cfg(test)]
mod properties {
    use std::collections::VecDeque;
    use super::*;

    pub fn code_decode<Code: UniversalCode>(vec: Vec<u64>) -> bool {
        let mut dv = VecDeque::<bool>::new();
        for &i in &vec {
            Code::encode(&mut dv, i + 1).unwrap();
        }

        let mut vec2 = Vec::<u64>::new();
        while let Ok(Some(i)) = Code::decode(&mut dv) {
            vec2.push(i - 1)
        }

        vec2 == vec
    }
}

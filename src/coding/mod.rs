//! Codes for data compression.
//!
//! These universal codes currently know how to encode to a `BitWrite`
//! and decode from a `BitRead`. However, the code that would use them
//! to implement compressed vectors and such isn’t written yet.

mod traits;
pub use self::traits::*;

mod unary;
pub use self::unary::*;

mod elias;
pub use self::elias::*;

mod fib;
pub use self::fib::*;

mod comma;
pub use self::comma::*;

mod trans;
pub use self::trans::*;

#[cfg(test)]
mod properties {
    use super::*;
    use std::collections::VecDeque;

    pub fn code_decode<Code: UniversalCode>(code: &Code, vec: Vec<u64>) -> bool {
        let mut dv = VecDeque::<bool>::new();
        for &i in &vec {
            code.encode(&mut dv, i + 1).unwrap();
        }

        let mut vec2 = Vec::<u64>::new();
        while let Ok(Some(i)) = code.decode(&mut dv) {
            vec2.push(i - 1)
        }

        vec2 == vec
    }
}

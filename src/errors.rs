use std::io::{Error, ErrorKind, Result};

pub fn out_of_bits<A>(who: &str) -> Result<A> {
    Err(Error::new(ErrorKind::InvalidInput,
                   format!("{}: could not decode: more bits expected",
                           who)))
}

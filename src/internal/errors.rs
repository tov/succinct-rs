use std::io::{Error, ErrorKind, Result};

pub fn out_of_bits<A>(who: &str) -> Result<A> {
    Err(Error::new(
        ErrorKind::UnexpectedEof,
        format!("{}: could not decode: more bits expected", who),
    ))
}

pub fn too_many_bits<A>(who: &str) -> Result<A> {
    Err(Error::new(
        ErrorKind::InvalidData,
        format!("{}: value too big for type", who),
    ))
}

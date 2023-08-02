use super::*;
use crate::{internal::errors::*, stream::*};

/// Lifts any code by adding one to each encoded value, and subtracting
/// one from each decoded value.
///
/// This is useful when the underlying code, like Elias codes, can’t handle 0s.
pub struct Lift0<Code: UniversalCode>(pub Code);

impl<Code: UniversalCode> UniversalCode for Lift0<Code> {
    fn encode<W: BitWrite>(&self, sink: &mut W, value: u64) -> Result<()> {
        if let Some(value) = value.checked_add(1) {
            self.0.encode(sink, value)
        } else {
            too_many_bits("Lift0::encode")
        }
    }

    fn decode<R: BitRead>(&self, source: &mut R) -> Result<Option<u64>> {
        match self.0.decode(source) {
            Ok(Some(n)) => Ok(Some(n - 1)),
            otherwise => otherwise,
        }
    }
}

use std::marker::PhantomData;

use super::*;
use stream::*;

/// Lifts any code by adding one to each encoded value, and subtracting
/// one from each decoded value.
///
/// This is useful when the underlying code, like Elias codes, can’t handle 0s.
pub struct Lift0<Code: UniversalCode>(PhantomData<Code>);

impl<Code: UniversalCode> UniversalCode for Lift0<Code> {
    fn encode<W: BitWrite>(sink: &mut W, value: u64) -> Result<()> {
        Code::encode(sink, value + 1)
    }

    fn decode<R: BitRead>(source: &mut R) -> Result<Option<u64>> {
        match Code::decode(source) {
            Ok(Some(n)) => Ok(Some(n - 1)),
            otherwise => otherwise,
        }
    }
}

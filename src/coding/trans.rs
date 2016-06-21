use std::marker::PhantomData;

use num::PrimInt;

use super::*;
use stream::*;

/// Lifts any code by adding one to each encoded value, and subtracting
/// one from each decoded value.
///
/// This is useful when the underlying code, like Elias codes, can’t handle 0s.
pub struct Lift0<Code: UniversalCode>(PhantomData<Code>);

impl<Code: UniversalCode> UniversalCode for Lift0<Code> {
    fn encode<W: BitWrite, N: PrimInt>(sink: &mut W, value: N) -> Result<()> {
        Code::encode(sink, value + N::one())
    }

    fn decode<R: BitRead, N: PrimInt>(source: &mut R) -> Result<Option<N>> {
        match Code::decode(source) {
            Ok(Some(n)) => Ok(Some(n - N::one())),
            otherwise => otherwise,
        }
    }
}

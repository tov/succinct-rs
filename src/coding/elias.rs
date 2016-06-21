use std::marker::PhantomData;
use std::mem;

use num::PrimInt;

use super::*;
use stream::*;

/// An Elias code.
///
/// Elias codes do not handle 0.
///
/// An Elias code first encodes the size of the number using some other
/// code—this is the `Header` parameter.
pub struct Elias<Header: UniversalCode>(PhantomData<Header>);

/// An Elias gamma code encodes the header in unary.
pub type Gamma = Elias<Unary>;

/// An Elias delta code encodes the header using the Elias gamma code.
pub type Delta = Elias<Gamma>;

impl<Header: UniversalCode> UniversalCode for Elias<Header> {
    fn encode<W: BitWrite, N: PrimInt>(sink: &mut W, value: N) -> Result<()> {
        assert!(value != N::zero(), "Elias codes do not handle 0");

        let nbits = 8 * mem::size_of::<N>() as u32 - value.leading_zeros() - 1;
        try!(Header::encode(sink, nbits));
        sink.write_int(nbits as usize, value)
    }

    fn decode<R: BitRead, N: PrimInt>(source: &mut R) -> Result<Option<N>> {
        if let Some(nbits) = try!(Header::decode(source)) {
            let low_bits: N = try!(source.read_int(nbits));
            Ok(Some(low_bits | (N::one() << nbits)))
        } else {
            Ok(None)
        }
    }
}

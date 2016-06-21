pub use std::io::Result;

use num::PrimInt;

use stream::*;

/// A universal code lets us encode arbitrary sized integers in a
/// self-delimiting code.
pub trait UniversalCode {
    /// Writes `value` to `sink`.
    fn encode<W: BitWrite, N: PrimInt>(sink: &mut W, value: N) -> Result<()>;

    /// Reads a value from `source`.
    ///
    /// `Ok(None)` indicates (benign) EOF.
    fn decode<R: BitRead, N: PrimInt>(source: &mut R) -> Result<Option<N>>;
    // TODO: bigint support
}

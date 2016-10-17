pub use std::io::Result;

use stream::*;

/// A universal code lets us encode arbitrary sized integers in a
/// self-delimiting code.
pub trait UniversalCode {
    /// Writes `value` to `sink`.
    fn encode<W: BitWrite>(&self, sink: &mut W, value: u64) -> Result<()>;

    /// Reads a value from `source`.
    ///
    /// `Ok(None)` indicates (benign) EOF.
    fn decode<R: BitRead>(&self, source: &mut R) -> Result<Option<u64>>;

    // TODO: bigint support
}

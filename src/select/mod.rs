//! Support for fast select queries.

mod bin_search;
pub use self::bin_search::*;

mod traits;
pub use self::traits::*;

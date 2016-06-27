//! Support for fast rank queries.

mod jacobson;
pub use self::jacobson::*;

mod traits;
pub use self::traits::*;

mod prim;

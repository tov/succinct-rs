//! Support for fast rank queries.

mod jacobson;
pub use self::jacobson::*;

mod rank9;
pub use self::rank9::*;

mod traits;
pub use self::traits::*;

mod prim;

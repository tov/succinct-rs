//! Codes used for data compression.

mod traits;
pub use self::traits::*;

mod unary;
pub use self::unary::*;

mod elias;
pub use self::elias::*;

mod trans;
pub use self::trans::*;

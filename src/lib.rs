//! Succinct Data Structures for Rust
//!
//! # Usage
//!
//! It’s [on crates.io](https://crates.io/crates/succinct), so you can add
//!
//! ```toml
//! [dependencies]
//! succinct = "*"
//! ```
//!
//! to your `Cargo.toml` and
//!
//! ```rust
//! extern crate succinct;
//! ```
//!
//! to your crate root.

#![warn(missing_docs)]

#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate num;

mod block_type;
pub use block_type::*;

mod int_vec;
pub use int_vec::{IntVec, IntVecBuilder};

mod bit_vector;
pub use bit_vector::{Rank, Select};

mod rank;
pub use rank::RankSupport;

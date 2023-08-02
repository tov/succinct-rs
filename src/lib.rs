//! Succinct data structures for Rust.
//!
//! So far we have:
//!
//!   - [bit vectors](bit_vector/struct.BitVec.html) and [bit
//!     buffers](stream/struct.BitBuffer.html);
//!   - [integer vectors](int_vector/struct.IntVec.html) with arbitrary-sized
//!     (1- to 64-bit) elements;
//!   - a variety of [universal codes](coding/index.html;
//!   - constant-time [rank](struct.JacobsonRank.html) queries; and
//!   - *O*(lg lg *n*)-time [select](struct.BinSearchSelect.html) queries
//!     based on binary search over ranks.
//!
//! # Usage
//!
//! It’s [on crates.io](https://crates.io/crates/succinct), so you can add
//!
//! ```toml
//! [dependencies]
//! succinct = "0.5.4"
//! ```
//!
//! to your `Cargo.toml`.

#![warn(missing_docs)]

#[macro_use]
mod macros;

mod internal;

pub mod broadword;
pub mod coding;
pub mod storage;
pub mod stream;

mod space_usage;
pub use crate::space_usage::SpaceUsage;

pub mod bit_vec;
pub use crate::bit_vec::{BitVec, BitVecMut, BitVecPush, BitVector};

pub mod int_vec;
pub use crate::int_vec::{IntVec, IntVecMut, IntVector};

pub mod rank;
pub use crate::rank::{BitRankSupport, JacobsonRank, Rank9};

pub mod select;
pub use crate::select::{BinSearchSelect, Select1Support};

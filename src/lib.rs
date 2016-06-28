//! Succinct data structures for Rust.
//!
//! This library is an early work in progress. So far we have:
//!
//!   - [integer vectors](int_vector/struct.IntVec.html) with arbitrary-sized
//!     (1- to 64-bit) elements;
//!   - [bit vectors](bit_vector/struct.BitVec.html) and [bit
//!     buffers](stream/struct.BitBuffer.html),
//!   - a variety of [universal codes](coding/index.html),
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
//! succinct = "0.3.2"
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

extern crate byteorder;
extern crate num;

#[cfg(test)]
extern crate quickcheck;

#[macro_use]
mod macros;

mod internal;

pub mod broadword;
pub mod coding;
pub mod storage;
pub mod stream;

mod space_usage;
pub use space_usage::SpaceUsage;

pub mod bit_vec;
pub use bit_vec::{BitVec, BitVecMut, BitVecPush, BitVector};

pub mod int_vec;
pub use int_vec::{IntVec, IntVecMut, IntVector};

pub mod rank;
pub use rank::{BitRankSupport, JacobsonRank, Rank9};

pub mod select;
pub use select::{Select1Support, BinSearchSelect};


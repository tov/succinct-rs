//! Succinct data structures for Rust.
//!
//! This library is a very early work in progress. So far we have:
//!
//!   - [integer vectors](vector/struct.IntVec.html) with arbitrary-sized
//!     (1- to 64-bit) elements;
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
//! succinct = "@VERSION@"
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

mod util;
mod errors;

pub mod storage;
pub mod stream;
pub mod coding;

pub mod bit_vector;
pub use bit_vector::{Bits, BitsMut, BitVector, BitVec};

pub mod int_vector;
pub use int_vector::{IntVector, IntVectorMut, IntVec};

pub mod rank;
pub use rank::JacobsonRank;

pub mod select;
pub use select::BinSearchSelect;

mod space_usage;
pub use space_usage::SpaceUsage;

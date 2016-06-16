//! Succinct data structures for Rust.
//!
//! This library is a very early work in progress. So far we have:
//!
//!   - [integer vectors](struct.IntVec.html) with arbitrary-sized
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

extern crate num;

mod int_vec;
pub use int_vec::{IntVec, IntVecBuilder};

mod storage;
pub use storage::{BitStore, BitStoreMut, BlockType};

mod rank;
pub use rank::{RankSupport, JacobsonRank};

mod select;
pub use select::{SelectSupport, BinSearchSelect};

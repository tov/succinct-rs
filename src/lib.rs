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

pub mod int_vec;
pub use int_vec::{IntVec, IntVecBuilder};

pub mod storage;
pub use storage::{BitStore, BlockType};

pub mod rank;
pub use rank::{RankSupport, JacobsonRank};

pub mod select;
pub use select::{SelectSupport, BinSearchSelect};

# Succinct Data Structures for Rust

[![Build Status](https://travis-ci.org/tov/succinct-rs.svg?branch=master)](https://travis-ci.org/tov/succinct-rs)
[![Crates.io](https://img.shields.io/crates/v/succinct.svg?maxAge=2592000)](https://crates.io/crates/succinct)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache_2.0-blue.svg)](LICENSE-APACHE)

This library is an early work in progress. So far we have:

  - integer vectors with arbitrary-sized (1- to 64-bit) elements;
  - bit vectors and bit buffers,
  - a variety of universal codes,
  - constant-time rank queries; and
  - *O*(lg lg *n*)-time select queries based on binary search over ranks.

## Usage

It’s [on crates.io](https://crates.io/crates/succinct), so you can add

```toml
[dependencies]
succinct = "0.3.2"
```

to your `Cargo.toml` and

```rust
extern crate succinct;
```

to your crate root.

## Credits

  - `IntVec` borrows some implementation techniques from
    [`nbitsvec`](https://crates.io/crates/nbits_vec). The main
    difference is that `nbitsvec` uses a `typenum` to put the element
    size (in bits) as a parameter to the vector type. Also, `nbitsvec`
    is likely to be faster.

  - Some of the API is inspired by
    [SDSL](https://github.com/simongog/sdsl-lite), a C++ succinct data
    structures library. It’s much more complete than `succinct`, and
    probably more correct and faster too.

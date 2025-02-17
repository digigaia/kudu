//!
//! This crate provides tools to create and sign EOS Signing Requests (ESR).
//!
//! ## Feature flags
//!
//! - `float128`: add support for a native `float128` type. This currently needs a nightly Rust
//!               version as `f128` support is still experimental. If this is not active,
//!               the `Float128` will still be available but as a `[u8; 16]` wrapper.

#![doc = include_str!("../TODO.md")]

#![cfg_attr(feature = "float128", feature(f128))]

pub mod signing_request;

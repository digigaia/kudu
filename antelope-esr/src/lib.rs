//!
//! This crate provides tools to create and sign EOS Signing Requests (ESR).
//!
//! ## Feature flags
//!
//! - `float128`: add support for the `float128` type, needs a nightly Rust version
//!               as `f128` support in is still experimental

#![doc = include_str!("../TODO.md")]

#![cfg_attr(feature = "float128", feature(f128))]

pub mod signing_request;

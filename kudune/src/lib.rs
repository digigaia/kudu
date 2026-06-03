// SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

//!
//! This crate provides a `kudune` executable that replicates the functionality of the
//! original [DUNES](https://github.com/AntelopeIO/DUNES) which has been deprecated.
//!

#![doc = include_str!("../README.md")]

pub mod command;
pub mod docker;
pub mod dune;
pub mod nodeconfig;
mod ratatui;
pub mod util;

pub use command::{DockerCommand, DockerCommandJson};
pub use docker::Docker;
pub use dune::{BuildOpts, Dune};
pub use nodeconfig::NodeConfig;

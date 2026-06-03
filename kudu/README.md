<!--
SPDX-FileCopyrightText: 2025, 2026 DigiGaia SCCL
SPDX-License-Identifier: AGPL-3.0-or-later
-->

<div align="center">

# Kudu

**A Rust library for interacting with Antelope blockchains**

[![Latest published Kudu version](https://img.shields.io/crates/v/kudu.svg)](https://crates.io/crates/kudu)
[![Documentation build status](https://img.shields.io/docsrs/kudu.svg)](https://docs.rs/kudu)
[![AGPLv3+ license](https://img.shields.io/crates/l/kudu)](#license)

</div>

Kudu is a library that provides data types and functions to interact with
[Antelope](https://antelope.io) blockchains.

It is subdivided into 3 main crates for now:
- [`kudu`](https://docs.rs/kudu): contains the core types and ABI functionality. It also provides the `kuduconv` CLI tool.
- [`kudu-esr`](https://docs.rs/kudu-esr): provides ESR (EOSIO Signing Request) utils.
- [`kudune`](https://docs.rs/kudune): is a CLI tool that helps you manage and run nodeos instances in Docker.
  It aims at replacing the deprecated [DUNES](https://github.com/AntelopeIO/DUNES) utility


## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md) for details.


## License

This project is licensed under the GNU Affero General Public License v3.0 or later - see the [LICENSE](../LICENSES/AGPL-3.0-or-later.txt) file for details.

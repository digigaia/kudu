<div align="center">

# Kudu

**A Rust library for interacting with Antelope blockchains**

[![Latest published Kudu version](https://img.shields.io/crates/v/kudu.svg)](https://crates.io/crates/kudu)
[![Documentation build status](https://img.shields.io/docsrs/kudu.svg)](https://docs.rs/kudu)
[![Apache 2.0 or MIT license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue.svg)](#license)

</div>

Kudu is a library that provides data types and functions to interact with
[Antelope](https://antelope.io) blockchains.

It is subdivided into 3 main crates for now:
- [`kudu`](https://docs.rs/kudu): contains the core types and ABI functionality. It also provides the `kuduconv` CLI tool.
- [`kudu-esr`](https://docs.rs/kudu-esr): provides ESR (EOSIO Signing Request) utils.
- [`kudune`](https://docs.rs/kudune): is a CLI tool that helps you manage and run nodeos instances in Docker.
  It aims at replacing the deprecated [DUNES](https://github.com/AntelopeIO/DUNES) utility


## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

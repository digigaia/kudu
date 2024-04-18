# TODO / FIXME

TODO IMPORTANT!!

After splitting workspace into different crates, do the following:
- review Cargo.toml for each of them and remove unused dependencies
- check for minimum rust version (I think there is a tool for that)
- review each file individually


## API DESIGN

- better error handling when constructing types. We should remove `assert`s and `panic` and use proper error types

- is `from_str` the best name for most of our types constructors? Reconsider disabling the clippy warning
  about it at the top of `antelope-{core,abi}/src/lib.rs`

- use `From` and `Into` traits for constructing base Antelope types

- `BinarySerializable` / `ABISerializable` need to use `StreamError` instead of `InvalidValue`

- clean abiencoder.rs

- investigate `snafu` instead of `thiserror` for errors
  - <https://www.reddit.com/r/rust/comments/dfs1zk/2019_q4_error_patterns_snafu_vs_errderive_anyhow/>
  - <https://dev.to/e_net4/migrating-from-quick-error-to-snafu-a-story-on-revamped-error-handling-in-rust-58h9>
  - <https://news.ycombinator.com/item?id=28802428>
  - <https://news.ycombinator.com/item?id=28800680>
  - <https://gist.github.com/quad/a8a7cc87d1401004c6a8973947f20365>
  - <https://stackoverflow.com/questions/60943851/how-do-you-see-an-errors-backtrace-when-using-snafu>

- rename encode/decode methods everywhere to be more specific, such as `bin_to_json`/`json_to_bin`, etc. (esp. in tests)

- investigate whether `color_eyre::Result` is the right result type for the library. Maybe we should use `std::Result` and reserve the usage of `color_eyre::Result` for the unittests?

- check <https://rust-lang.github.io/api-guidelines/checklist.html>

### Investigate Serde

- check whether ABIEncoder would be better written as a Serde serializer

- rename ABISerializable to ABISerialize to be consistent with `serde`. Check other nomenclature as well.
  are we sure about this?
  Also: make sure we have a trait for this and implement it on all types? for now Name implements decode/encode as normal methods, not as trait methods


## CORRECTNESS / TESTING

- check whether we can fix this test for abieos float printing:
  `check_round_trip2(abi, "float64", "151115727451828646838272.0", "000000000000C044", "151115727451828650000000"`

- IMPORTANT: check symbol name validation, in EOS it can overflow here:
  <https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/include/eosio/chain/symbol.hpp#L34>

- do we allow constructing non-normalized names?
  see: tests/abieos_test.rs:402 vs.

- check for `unwrap` and `panic!` and `assert` everywhere

- check abieos/test.cpp to ensure we cover also all the error cases with proper error messages

- do the other tests from here: <https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py>


## MISC

- implement `Debug` and `Display` trait for all basic types

- check whether we should declare common dependencies (like `serde_json`) in the workspace `Cargo.toml`
  or in each sub-crate

- for Name type: check unittests and validity of non-normalized names

- crypto primitives do not implement WebAuthn key and signatures yet

- implement action_result in abi and abi_parser

- check with <https://crates.io/crates/antelope> whether we can get the crate name

- do we want to use the `base64` crate with the URL_SAFE engine or do we keep our own (smaller and simpler) implementation?

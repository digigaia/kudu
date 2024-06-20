# TODO / FIXME

TODO IMPORTANT!!

After splitting workspace into different crates, do the following:
- review Cargo.toml for each of them and remove unused dependencies
- check for minimum rust version (I think there is a tool for that)
- review each file individually


## API DESIGN

- look at builder pattern / fluent interface for specifiying EncodeOptions in ESR
eg: settings paragraph in https://github.com/tesselode/kira/releases/tag/v0.9.0
https://docs.rs/builder-pattern/latest/builder_pattern/
https://zerotomastery.io/blog/rust-struct-guide/

- clean abi.rs

- try defining the `ABISerializable` trait and implement it for all types, then replace the `AntelopeValue` struct with just the implementation of the base types
  -> could something like this help? https://www.reddit.com/r/rust/comments/1d3b356/my_new_favorite_way_to_represent_asts_or_any/

- rename encode/decode methods everywhere to be more specific, such as `bin_to_json`/`json_to_bin`, etc. (esp. in tests)

- check <https://rust-lang.github.io/api-guidelines/checklist.html>

### Error Handling

- better error handling when constructing types. We should remove `assert`s and `panic` and use proper error types

- `BinarySerializable` / `ABISerializable` need to use `StreamError` instead of `InvalidValue`

- it seems that `binaryserializable::SerializeError` might be overused, check that all error
  definitions and usages are OK

- add #[with_location] to all error types derived with Snafu
  with_location doesn't currently work with AntelopeValue error (seemingly because of the visibility attr?)
- define specific error to abi.rs, do not reuse InvalidValue for it

### AntelopeValue

- is `from_str` the best name for most of our types constructors? Reconsider disabling the clippy warning
  about it at the top of `antelope-{core,abi}/src/lib.rs`

- use `From` and `Into` traits for constructing base Antelope types

### Investigate Serde

- check whether ABIEncoder would be better written as a Serde serializer

- rename ABISerializable to ABISerialize to be consistent with `serde`. Check other nomenclature as well.
  are we sure about this?
  Also: make sure we have a trait for this and implement it on all types? for now Name implements decode/encode as normal methods, not as trait methods


## CORRECTNESS / TESTING

- check whether we can fix this test for abieos float printing:
  `check_round_trip2(abi, "float64", "151115727451828646838272.0", "000000000000C044", "151115727451828650000000"`

- do we allow constructing non-normalized names?
  see: tests/abieos_test.rs:402 vs.
  for Name type: check unittests and validity of non-normalized names

- check for `unwrap` and `panic!` and `assert` everywhere

- check abieos/test.cpp to ensure we cover also all the error cases with proper error messages

- have some tests for `APIClient`, think how to do this smartly to not pound the API server

- check tests in https://github.com/AntelopeIO/leap/blob/main/unittests/abi_tests.cpp
  - at the end, there are tests about action results

- do the other tests from here: <https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py>

- check other tests and ideas from: https://github.com/wharfkit/antelope


## MISC

- check <https://rustprojectprimer.com/>

- implement `Debug` and `Display` trait for all basic types

- document everything, also use boxes to show structure in source code files (ie: trait impls, etc. see: Symbol.rs as an example)

- check with <https://crates.io/crates/antelope> whether we can get the crate name

- investigate <https://github.com/eosrio/rs-abieos>

- report bug for wharfkit.request creation: duplicate context_free_actions, missing context_free_data
  https://github.com/wharfkit/signing-request/blob/master/src/signing-request.ts#L410
  see tx def: https://docs.eosnetwork.com/docs/latest/advanced-topics/transactions-protocol/


## MISSING FEATURES

- crypto primitives do not implement WebAuthn key and signatures yet

- implement action_result in abi and abi_parser

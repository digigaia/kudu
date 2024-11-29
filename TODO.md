# TODO / FIXME

TODO IMPORTANT!!

After splitting workspace into different crates, do the following:
- review Cargo.toml for each of them and remove unused dependencies
- check for minimum rust version (use `cargo-msrv`)
- review each file individually

Make sure that every item on this page gets a corresponding entry in the ARCHITECTURE.md
file once they are implemented.


## API DESIGN

- clean abi.rs
  - check use and handling of binary extension

- try defining the `ABISerializable` trait and implement it for all types, then replace
  the `AntelopeValue` struct with just the implementation of the base types
  (note: we might still need AntelopeValue, maybe rename it to AntelopeVariant)

  for ESR: <https://github.com/AntelopeIO/spring/blob/main/libraries/chain/include/eosio/chain/transaction.hpp#L53>
  ```
  pub struct TransactionHeader {
      expiration: TimePointSec,
      ref_block_num: u16,
      ref_block_prefix: u32,
      max_net_usage_words: usize, // FIXME: check this type
      // etc...
  }
  ```

- check <https://rust-lang.github.io/api-guidelines/checklist.html>

### Naming

- rename `TypeNameRef` to `TypeName` (?)

### Error Handling


### Investigate Serde

- check whether ABIEncoder would be better written as a Serde serializer

- rename ABISerializable to ABISerialize to be consistent with `serde`. Check other nomenclature as well.
  are we sure about this?
  Also: make sure we have a trait for this and implement it on all types? for now Name implements decode/encode as normal methods, not as trait methods


## CORRECTNESS / TESTING

- IMPORTANT
  unittests for base types should have at least the following:
  - basic functionality
  - invalid values
  - (de)serialization to JSON

- do we allow constructing non-normalized names?
  see: tests/abieos_test.rs:402 vs.
  for Name type: check unittests and validity of non-normalized names

- check for `unwrap` and `panic!` and `assert` everywhere

- check abieos/test.cpp to ensure we cover also all the error cases with proper error messages

- have some tests for `APIClient`, think how to do this smartly to not pound the API server

- check tests in <https://github.com/AntelopeIO/leap/blob/main/unittests/abi_tests.cpp>
  - at the end, there are tests about action results

- do the other tests from here: <https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py>

- check other tests and ideas from: <https://github.com/wharfkit/antelope>

- replace links and refs from `AntelopeIO/leap` to `AntelopeIO/spring`


## PERFORMANCE

- use a small string library so that ABIs have a much better cache locality

- try using a `BTreeMap` or some other map that has better cache locality


## MISC

- include `ARCHITECTURE.md` somewhere in the docs

- investigate rust libs found here: <https://onblock.dev/communicating-with-the-wax-blockchain>

- check <https://rustprojectprimer.com/>

- check <https://kerkour.com/rust-how-to-organize-large-workspaces>
  <https://www.reddit.com/r/rust/comments/1e30mkl/how_to_organize_large_rust_codebases/>

- implement `Debug` and `Display` trait for all basic types

- document everything, also use boxes to show structure in source code files (ie: trait impls, etc. see: Symbol.rs as an example)

- check with <https://crates.io/crates/antelope> whether we can get the crate name
  alternatively, find another name: kudu, impala, tsessebe, etc. see: <https://africafreak.com/fastest-african-antelope> for fast antelopes :)

- investigate <https://github.com/eosrio/rs-abieos>

- report bug for wharfkit.request creation: duplicate context_free_actions, missing context_free_data
  <https://github.com/wharfkit/signing-request/blob/master/src/signing-request.ts#L410>
  see tx def: <https://docs.eosnetwork.com/docs/latest/advanced-topics/transactions-protocol/>


## MISSING FEATURES

- add note that the execution time of the various methods is not time bounded and recursive
  functions do not have a max depth that is checked either.
  This could be something added at a later time via a feature flag (eg: `hardened`)

- crypto primitives do not implement WebAuthn key and signatures yet

- implement action_result in abi and abi_parser

- update from Leap to Spring

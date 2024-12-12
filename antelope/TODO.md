# TODO / FIXME

- write `[tracing_test]` attr macro that makes a `[test]` and calls `tracing_init()` at the beginning
- add `[derive(Serialize)]` to all base types

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

### Naming

- rename `BlockTimestampType` to `BlockTimestamp` ?

- rename `TypeNameRef` to `TypeName` (?)


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

- have some tests for `APIClient`, think how to do this smartly to not pound the API server

- check abieos/test.cpp to ensure we cover also all the error cases with proper error messages

- check tests in <https://github.com/AntelopeIO/spring/blob/main/unittests/abi_tests.cpp>
  - at the end, there are tests about action results

- check other tests and ideas from: <https://github.com/wharfkit/antelope>, e.g.:
  <https://github.com/wharfkit/antelope/blob/master/test/chain.ts>

## PERFORMANCE

- use a small string library so that ABIs have a much better cache locality

- try using smallvec/tinyvec for the same reasons as small string, esp. on vectors that are
  empty most of the time, eg: extensions, etc.

- try using a `BTreeMap` or some other map that has better cache locality

- check if anything from this [reddit thread about `serde_json`](https://www.reddit.com/r/rust/comments/w3q1oq/things_i_wish_i_had_known_about_serde_json/) applies

- serializing bytes into a binary stream with serde currently calls `serialize_seq` and `serialize_u8` for each byte.
  make sure that this actually gets inlined properly so that for instance serializing a `Checksum256` doesn't end up
  calling a function 256 times, and also we should be able to memcpy all of it as well instead of byte per byte

  the alternative would be to make a newtype for `Bytes` instead of aliasing it to `Vec<u8>` and then we could have a
  specific implementation of `Serialize` for it

  use the Rust playground to check it: <https://play.rust-lang.org/>


## MISC

- investigate <https://github.com/eosrio/rs-abieos>


## MISSING FEATURES

- crypto primitives do not implement WebAuthn key and signatures yet

- add note that the execution time of the various methods is not time bounded and recursive
  functions do not have a max depth that is checked either.
  This could be something added at a later time via a feature flag (eg: `hardened`)

- implement action_result in abi and abi_parser
  see: <https://github.com/AntelopeIO/spring/commit/7da37b6bc41a63a9eaef5e79ff7aaf2aea854826#diff-a7893952d8a2b33ddc5b3c89250729ea6961784c8b9300a39f187a7357cc3149R165>

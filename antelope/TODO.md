# TODO / FIXME

- write `[tracing_test]` attr macro that makes a `[test]` and calls `tracing_init()` at the beginning
  even better: `[antelope_test]` that makes a `[test]` that returns a `Result<()>`, calls `tracing_init()`
  at the beginning() and returns `Ok(())` at the end.
- add `[derive(Serialize)]` to all base types

- `SerializeEnum` should use the inner type as snake-case discriminant instead of the identifier
  -> remove all the serde rename after that

- write a note with the difference in behavior between this and the C++ Antelope version
  - hex numbers are lowercase whereas C++ outputs in upper case
  - C++ outputs i64 and u64 as double-quoted

## API DESIGN

- clean abi.rs
  - check use and handling of binary extension
  - check whether we really need `AntelopeValue`, because if not then it seems we should be able
    to remove it altogether.

- define an attr macro for declaring contracts (such as in chain.rs) like so:
  ```
  #[contract(account="eosio.token", name="transfer")]
  pub struct Transfer {
      pub from: Name,
      pub to: Name,
      pub quantity: Asset,
      pub memo: String,
  }
  ```

- implement `From` traits for base types everywhere it makes sense, and `TryFrom` too

- clean/properly order imports in all file (maybe wait for Rust 2024 edition as it seems to
  correspond better to the style we like)

### Naming

- to_hex -> hex representation of binary data, to_bin -> binary data itself (ie: `vec<u8>`)

- rename `BinarySerializable` to `ABISerializable`?

### Investigate Serde

- deprecate/remove non human-readable impls for Serialize/Deserialize types


## CORRECTNESS / TESTING

- IMPORTANT
  unittests for base types should have at least the following:
  - basic functionality
  - invalid values
  - (de)serialization to JSON

- have some tests for `APIClient`, think how to do this smartly to not pound the API server

- check abieos/test.cpp to ensure we cover also all the error cases with proper error messages

- check float128 support
  maybe try to have float128 support on stable as we only need the hex representation of f128
  so we could have a stub for that type instead of the rust primitive which is only available on nightly

- check tests in <https://github.com/AntelopeIO/spring/blob/main/unittests/abi_tests.cpp>
  - at the end, there are tests about action results

- check other tests and ideas from: <https://github.com/wharfkit/antelope>, e.g.:
  <https://github.com/wharfkit/antelope/blob/master/test/chain.ts>


## PERFORMANCE

- switch reqwest with ureq

- use a small string library so that ABIs have a much better cache locality

- try using smallvec/tinyvec for the same reasons as small string, esp. on vectors that are
  empty most of the time, eg: extensions, etc. or only contain 1 or 2 elements,
  eg: `Action::authorization`, etc.

- try using a `BTreeMap` or some other map that has better cache locality, or a faster hash,
  like: <https://github.com/rust-lang/rustc-hash>

- use the Rust playground to check code: <https://play.rust-lang.org/>


## MISC

- investigate <https://github.com/eosrio/rs-abieos>


## MISSING FEATURES

- crypto primitives do not implement WebAuthn key and signatures yet

- implement action_result in abi and abi_parser
  see: <https://github.com/AntelopeIO/spring/commit/7da37b6bc41a63a9eaef5e79ff7aaf2aea854826#diff-a7893952d8a2b33ddc5b3c89250729ea6961784c8b9300a39f187a7357cc3149R165>

## SECURITY CONSIDERATIONS

- add note that the execution time of the various methods is not time bounded and recursive
  functions do not have a max depth that is checked either.
  This could be something added at a later time via a feature flag (eg: `hardened`)

- think about `serde_json::Value::Number` size and whether we're good with it

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

- `BinarySerializable` trait methods namings:
  `encode()` -> `to_bin()`
  `decode()` -> `from_bin()`

- rename `BinarySerializable` to `ABISerializable`?

### Investigate Serde

- deprecate/remove non human-readable impls for Serialize/Deserialize types

- remove hex_to_boxed_array?

## CORRECTNESS / TESTING

- IMPORTANT
  unittests for base types should have at least the following:
  - basic functionality
  - invalid values
  - (de)serialization to JSON

- review TimePoint types:
  - is the inner type the number of microseconds or milliseconds?
  - should we return a `Result` instead of an `Option` on constructors? This would be more consistent with other types
    if so, check in the tests and replace the `unwrap` with `?`
  - do we really need the from/into from u32/u64? It would be better to have a named constructor,
    ie: TimePoint::from_millis
    also: these conversions need to be fallible, ie: TimePoint(u32::MAX) does not really make sense
  - check that downcasting u64 to u32 is ok everywhere

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

- implement action_result in abi and abi_parser
  see: <https://github.com/AntelopeIO/spring/commit/7da37b6bc41a63a9eaef5e79ff7aaf2aea854826#diff-a7893952d8a2b33ddc5b3c89250729ea6961784c8b9300a39f187a7357cc3149R165>

## SECURITY CONSIDERATIONS

- add note that the execution time of the various methods is not time bounded and recursive
  functions do not have a max depth that is checked either.
  This could be something added at a later time via a feature flag (eg: `hardened`)

- think about `serde_json::Value::Number` size and whether we're good with it

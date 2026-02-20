# TODO / FIXME

## API DESIGN

NOTE: this should be fixed, or at least a resolution for this should be decided and
      documented in ARCHITECTURE.md before making a 1.0 release

- clean abi.rs

- the `ABISerializable` trait or the `ByteStream` struct needs to be revised:
  currently, `from_bin()` needs a `ByteStream` however the latter owns its data,
  meaning that if we only have a `&[u8]` we need to make a copy of the whole data
  before deserializing it.
  In other words, the choice is:
  - `ABISerializable::from_bin` needs to take `&[u8]` as input
    that would be the most generic, but then we reading from a bytestream would be awkward as
    we can't advance its cursor (is this actually really needed?)
  - a read-only `ByteStream` needs to be able to be cheaply created from `&[u8]`
  - (maybe?) we introduce a new `Cursor` struct (trait?) that can be created either from
    a `&[u8]` or by a `ByteStream`

- find a way to declare extension fields on native Rust structs. We can easily
  annotate them using attributes that are recognized by the `derive(ABISerializable)`
  macro but what should the implementation be like?

- add `impl Debug`/`impl Display` for the `contract` derive macro

- rename `ByteStream::data()` to `ByteStream::as_bytes()`

## CORRECTNESS / TESTING

- have some tests for `APIClient`, think how to do this smartly to not pound the API server
  use an http mocking library: `httpmock`, `wiremock`, `mockito`. `httpmock` has both async/sync
  API, `mockito` seems not so maintained? See comparison:
  <https://github.com/LukeMathWalker/wiremock-rs?tab=readme-ov-file#prior-art>
  alternatively, use a crate like `mockall` to mock directly the `APIClient` object
  also, see this about mocking: <https://borretti.me/article/two-years-of-rust> / <https://www.reddit.com/r/rust/comments/1jzwer2/two_years_of_rust/>

- check abieos/test.cpp to ensure we cover also all the error cases with proper error messages

- check other tests and ideas from: <https://github.com/wharfkit/antelope>, e.g.:
  <https://github.com/wharfkit/antelope/blob/master/test/chain.ts>

- check [Pitfalls of safe Rust](https://corrode.dev/blog/pitfalls-of-safe-rust/)
  [[reddit](https://www.reddit.com/r/rust/comments/1jqqzxb/pitfalls_of_safe_rust/)]


## PERFORMANCE

- use a small string library so that ABIs have a much better cache locality
  eg: "german strings": <https://cedardb.com/blog/german_strings/>

- try using smallvec/tinyvec for the same reasons as small string, esp. on vectors that are
  empty most of the time, eg: extensions, etc. or only contain 1 or 2 elements,
  eg: `Action::authorization`, etc. (`ecow` seems like a good choice but investigate further)

- try using a `BTreeMap` or some other map that has better cache locality, or a faster hash,
  like: <https://github.com/rust-lang/rustc-hash> or ahash

- investigate `fastrace` to replace `tokio-rs/tracing`:
  <https://www.reddit.com/r/rust/comments/1jh2fzg/fastrace_a_modern_approach_to_distributed_tracing/>

- check compilation options for kudu binaries: <https://github.com/johnthagen/min-sized-rust>

- use the Rust playground to check code: <https://play.rust-lang.org/>


## MISC

- investigate `darling` crate to help with derive macros, here's a
  [small example](https://github.com/imbolc/rust-derive-macro-guide)
  maybe even better: `pastey`, `crabtime` (crabtime does look good!)
  other choice: [`attrs`](https://www.reddit.com/r/rust/comments/1joffd5/announcing_attrs_a_parsercombinator_library_for/)

- clean/properly order imports in all file (maybe wait for Rust 2024 edition and use rustfmt
  as it seems to correspond better to the style we like)

- upgrade crypto crates once they have been released (see: https://github.com/RustCrypto/hashes)


## MISSING FEATURES

- crypto primitives do not implement WebAuthn key and signatures yet

- ensure action_result is properly implemented in abi and abi_parser
  see: <https://github.com/AntelopeIO/spring/commit/7da37b6bc41a63a9eaef5e79ff7aaf2aea854826#diff-a7893952d8a2b33ddc5b3c89250729ea6961784c8b9300a39f187a7357cc3149R165>
  see: <https://github.com/wharfkit/antelope/commit/bfd07762f329cda02e8470ce6c18eb4543a74253>

- add support to ABI for the `calls` field in 1.3
  see: <https://github.com/wharfkit/antelope/pull/120/commits/966fbde30242fbb288572896c27e601496529c83>

- implement new ABI types: `bitset` and fixed-size arrays
  see: <https://github.com/AntelopeIO/spring/wiki>

- add preloaded ABIs into `kuduconv`, also have it auto-detect the abi from the preloaded ones if none specified


## SECURITY CONSIDERATIONS

- add note that the execution time of the various methods is not time bounded and recursive
  functions do not have a max depth that is checked either.
  This could be something added at a later time via a feature flag (eg: `hardened`)

- think about `serde_json::Value::Number` size and whether we're good with it

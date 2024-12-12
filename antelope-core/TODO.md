# TODO / FIXME

- write `[tracing_test]` attr macro that makes a `[test]` and calls `tracing_init()` at the beginning
- add `[derive(Serialize)]` to all base types

## NAMING

- rename `BlockTimestampType` to `BlockTimestamp` ?

## CORRECTNESS / TESTING

- IMPORTANT
  unittests for base types should have at least the following:
  - basic functionality
  - invalid values
  - (de)serialization to JSON

- have some tests for `APIClient`, think how to do this smartly to not pound the API server

- check other tests and ideas from: <https://github.com/wharfkit/antelope>, e.g.:
  <https://github.com/wharfkit/antelope/blob/master/test/chain.ts>

## MISSING FEATURES

- crypto primitives do not implement WebAuthn key and signatures yet

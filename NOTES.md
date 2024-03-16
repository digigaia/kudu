# NOTES / RESOURCES

## Tests

Tests are sourced from the following locations:

- <https://github.com/AntelopeIO/abieos/>
- <https://github.com/AntelopeIO/abieos/blob/main/src/test.cpp#L577>
- <https://github.com/AntelopeIO/abieos/test/>

- <https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py>

- <https://github.com/pinax-network/antelope.rs>

## Antelope source files locations

Types:
- <https://github.com/AntelopeIO/leap/blob/main/libraries/chain/include/eosio/chain/name.hpp>
- <https://github.com/AntelopeIO/leap/blob/main/libraries/chain/include/eosio/chain/symbol.hpp>
- etc.

Crypto types:
- <https://github.com/AntelopeIO/leap/blob/main/libraries/libfc/src/crypto/>

### ABI related files

- <https://github.com/AntelopeIO/leap/blob/main/libraries/chain/include/eosio/chain/abi_def.hpp>

TODO! check with

- <https://github.com/AntelopeIO/abieos/blob/main/include/eosio/abi.hpp>

the latter one seems more strict, for instance for the tests here:
<https://github.com/AntelopeIO/abieos/blob/main/src/test.cpp#L1008>


## ESR - EOSIO Signing Requests

Spec is at: <https://github.com/eosio-eps/EEPs/blob/master/EEPS/eep-7.md>

<https://github.com/wharfkit/signing-request>

## Design decisions

This part lists the design decisions that went into the library. It helps document
some decisions and their rationale and keep a trace so we don't have to ask the
same question or wonder why some choices have been made over and over again.

**TODO!**

- why we have `ByteStream` instead of using something like the `bytes` crate
  operation on bytes are infallible, and we are fine with having write being infallible
  (ie: we can grow a vec or append to a file we are writing to), but we want reads to be
  able to fail, ie: when we are at the end of the stream/file
  TODO: make sure this holds and see whether we can use the `bytes` crate anyway, it has
        some nice features

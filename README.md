# Rust Antelope utils


## TODO / FIXME

- IMPORTANT: check symbol name validation, in EOS it can overflow here:
  https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/include/eosio/chain/symbol.hpp#L34


- we should `pub use serde_json::Value as antelope::Variant` or find another type

- rename `ABIEncoder::from_abi` to `ABIEncoder::with_abi` ?

- rename ABISerializable to ABISerialize to be consistent with `serde`. Check other nomenclature as well.

- use `From` and `Into` traits for constructing base Antelope types

- check for `unwrap` and `panic!` everywhere

- better error handling when constructing types. We should remove `assert`s and `panic` and use proper error types

// TODO: do the other tests from here: https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py
// missing for now:
//  - UnixTimestamp
//  - TimePoint


## NOTES

tests locations

https://github.com/AntelopeIO/abieos/
https://github.com/AntelopeIO/abieos/src/test.cpp
https://github.com/AntelopeIO/abieos/test/

https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py

https://github.com/AntelopeIO/abieos/blob/main/src/test.cpp#L577

https://github.com/pinax-network/antelope.rs

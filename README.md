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

- use https://docs.rs/bytes/latest/bytes/buf/trait.BufMut.html instead of ByteStream

- check that bin encoding for Bytes is correct (len as varuint32?)

- check abieos/test.cpp to ensure we cover also all the error cases with proper error messages

- move all bin_to_hex and hex_to_bin functions into a dedicated hexutils crate (with dedicated error type)

- check whether we can fix this test for abieos float printing:
  `check_round_trip2(abi, "float64", "151115727451828646838272.0", "000000000000C044", "151115727451828650000000"`

- check if we need to Box a few types in the AntelopeType enum (eg: PublicKey, Signature,...)

- rename keytype.suffix() to keytype.prefix()

- crypto primitives do not implement WebAuthn key and signatures yet

// TODO: do the other tests from here: https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py
// missing for now:
//  - UnixTimestamp
//  - TimePoint

- rename AntelopeType to AntelopeValue, use variant names as AntelopeType, and rewrite method that take a typename as str to method taking the typename as AntelopeType

## NOTES

tests locations

https://github.com/AntelopeIO/abieos/
https://github.com/AntelopeIO/abieos/src/test.cpp
https://github.com/AntelopeIO/abieos/test/

https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py

https://github.com/AntelopeIO/abieos/blob/main/src/test.cpp#L577

https://github.com/pinax-network/antelope.rs

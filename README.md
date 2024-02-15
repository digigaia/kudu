# Rust Antelope utils


## TODO / FIXME

### API DESIGN

- rename `ABIEncoder::from_abi` to `ABIEncoder::with_abi` ?

- rename ABISerializable to ABISerialize to be consistent with `serde`. Check other nomenclature as well.

- better error handling when constructing types. We should remove `assert`s and `panic` and use proper error types

- use `From` and `Into` traits for constructing base Antelope types

- use https://docs.rs/bytes/latest/bytes/buf/trait.BufMut.html instead of ByteStream

- rename AntelopeType to AntelopeValue, use variant names as AntelopeType, and rewrite method that take a typename as str to method taking the typename as AntelopeType

- clean abiencoder.rs

- investigate whether color_eyre::Result is the right result type for the library. Maybe we should use std::Result and reserve the usage of color_eyre::Result for the unittests?


### CORRECTNESS / TESTING

- check whether we can fix this test for abieos float printing:
  `check_round_trip2(abi, "float64", "151115727451828646838272.0", "000000000000C044", "151115727451828650000000"`

- IMPORTANT: check symbol name validation, in EOS it can overflow here:
  https://github.com/AntelopeIO/leap/blob/6817911900a088c60f91563995cf482d6b380b2d/libraries/chain/include/eosio/chain/symbol.hpp#L34

- check for `unwrap` and `panic!` everywhere

- check abieos/test.cpp to ensure we cover also all the error cases with proper error messages

- do the other tests from here: https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py


### MISC

- for Name type: implement .prefix() and check unittests and validity of non-normalized names

- check if we need to Box a few types in the AntelopeType enum (eg: PublicKey, Signature,...)
  maybe we also want to box i128 types, they would double the size of the enum while being seldom(?) used

- crypto primitives do not implement WebAuthn key and signatures yet

- implement action_result in abi and abi_parser


## NOTES

tests locations

https://github.com/AntelopeIO/abieos/
https://github.com/AntelopeIO/abieos/src/test.cpp
https://github.com/AntelopeIO/abieos/test/

https://github.com/FACINGS/pyntelope/blob/main/tests/unit/types_test.py

https://github.com/AntelopeIO/abieos/blob/main/src/test.cpp#L577

https://github.com/pinax-network/antelope.rs

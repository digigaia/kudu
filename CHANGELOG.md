
# 0.2 - Python bindings

## General enhancements

 - cleanup API around ABI/ABIProvider: there is now a global registry for preloaded ABIs, which doesn't require
   us to pass ABIs around for a lot of methods that need them (encoding/decoding JSON <-> binary).
   It is still possible to explicitly pass an ABI if needed.

 - `kuduconv` does not require to pass an ABI explicitly anymore, if will select a preloaded one automatically
   that matches the type being converted

 - new types/structs: `Transaction`, that can also sign if given a private key


## Kudune enhancements

 - it can compile Spring and CDT instead of downloading packages when building an image
 - it can run on MacOS (tested with Orbstack), will use an amd64 base image
 - increased default cpu max usage time for transactions to be able to run on lower-power
   machines (eg: CI, emulated AMD64 on Apple silicon, etc.


## Python bindings

This release sees the introduction of the kudu python bindings (in the `kudu-py` subfolder)

For now, there are a few classes for pushing transactions to a running node:
`Name`, `PermissionLevel`, `Action`, `Transaction`, `SignedTransaction`, `APIClient`, `PublicKey`, `PrivateKey`.

There is also a very basic, very insecure wallet for managing keys (useful for running tests and during dev).

Bindings are not complete yet but they can already successfully run the `kudu-py/test_chain.py` test
that does the following:

- create a new docker container with a fresh install of nodeos
- start nodeos and bootstrap a fully running Vaulta chain
- create a few users
- create a new token
- distribute some of those tokens to the users
- have those users transfer tokens to each other


# 0.1 - Initial release

Initial release of the `Kudu` Rust library for interacting with Antelope blockchains. At the moment,
only Vaulta is explicitly supported but the aim it to support the entire family of Antelope-compatible
blockchains.

It is subdivided into 3 main crates for now:

- `kudu`: contains the core types and ABI functionality. It also provides the `kuduconv` CLI
          tool (similar to `abieos`).
- `kudu-esr`: provides ESR (EOSIO Signing Request) utils.
- `kudune`: is a CLI tool that helps you manage and run nodeos instances in Docker.
            It aims at replacing the deprecated `DUNES` utility

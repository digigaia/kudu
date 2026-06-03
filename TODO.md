<!--
SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
SPDX-License-Identifier: AGPL-3.0-or-later
-->

# TODO / FIXME

This is a general list that applies to the entire package. For crate specific items,
please look at `README.md` / `TODO.md` inside each crate directory.

Make sure that every item on this page gets a corresponding entry in the ARCHITECTURE.md
file once they are implemented.


## API DESIGN

- check <https://rust-lang.github.io/api-guidelines/checklist.html>


## DOCUMENTATION

- read the [rustdoc book](https://doc.rust-lang.org/stable/rustdoc/), see how we can enhance
  the documentation

- make modules private at the crate level so that items re-exported from them appear directly
  as top-level structs/traits/etc. instead of being listed in the "Re-exports" section

  more generally, check all modules visibility


## CORRECTNESS / TESTING

- check for `unwrap` and `panic!` and `assert` everywhere
  use the following to forbid `unwrap()` and allow them on a case-by-case basis
  ```
  [lints.clippy]
  unwrap_used = "deny"
  ```

- review singletons usage in tests


## MISC

- Transition from EOS -> Vaulta
  - rename project name EOS -> Vaulta
  - check that system contracts properly use the Vaulta ones
  - verify accounts used (ie: eosio -> core.vaulta?)
  - check token name is `A` now instead of `EOS` (or `SYS`) => upgrade to system contracts 3.10.0
  - check following links:
    - https://github.com/AntelopeIO/spring/pull/1536
    - https://github.com/AntelopeIO/spring/blob/main/tutorials/bios-boot-tutorial/bios-boot-tutorial.py
    - https://github.com/VaultaFoundation/system-contracts/pull/206

- add license before publishing, also in all cargo.toml and pyproject.toml files

- setup CI using GitHub actions before publishing

- investigate https://github.com/release-plz/release-plz for releasing new versions

- include `ARCHITECTURE.md` somewhere in the docs

- investigate rust libs found here: <https://onblock.dev/communicating-with-the-wax-blockchain>

- check <https://rustprojectprimer.com/>

- check <https://kerkour.com/rust-how-to-organize-large-workspaces>
  <https://www.reddit.com/r/rust/comments/1e30mkl/how_to_organize_large_rust_codebases/>

- check <https://kerkour.com/rust-production-checklist> for advice. In particular:
  - release configuration
  - replace the global allocator by mimalloc

- implement `Debug` and `Display` trait for all basic types

- implement `Deref` for newtypes, this way we automatically expose most of the API of the underlying type.
  We still need to implement `Display`, `From`, etc. manually

- document everything, also use boxes to show structure in source code files (ie: trait impls, etc. see: Symbol.rs as an example)

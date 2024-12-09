# TODO / FIXME

This is a general list that applies to the entire package. For crate specific items,
please look at `README.md` / `TODO.md` inside each crate directory.

Make sure that every item on this page gets a corresponding entry in the ARCHITECTURE.md
file once they are implemented.


## API DESIGN

- check <https://rust-lang.github.io/api-guidelines/checklist.html>


## DOCUMENTATION

- make modules private at the crate level so that items re-exported from them appear directly
  as top-level structs/traits/etc. instead of being listed in the "Re-exports" section


## CORRECTNESS / TESTING

- check for `unwrap` and `panic!` and `assert` everywhere


## MISC

- add license before publishing

- create an `antelope` crate to re-export all useful structs from the other crates and gather
  documentation in a single place (?)

- include `ARCHITECTURE.md` somewhere in the docs

- investigate rust libs found here: <https://onblock.dev/communicating-with-the-wax-blockchain>

- check <https://rustprojectprimer.com/>

- check <https://kerkour.com/rust-how-to-organize-large-workspaces>
  <https://www.reddit.com/r/rust/comments/1e30mkl/how_to_organize_large_rust_codebases/>

- implement `Debug` and `Display` trait for all basic types

- document everything, also use boxes to show structure in source code files (ie: trait impls, etc. see: Symbol.rs as an example)

- check with <https://crates.io/crates/antelope> whether we can get the crate name
  alternatively, find another name: kudu, impala, tsessebe, etc. see: <https://africafreak.com/fastest-african-antelope> for fast antelopes :)

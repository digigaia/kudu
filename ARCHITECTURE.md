# ARCHITECTURE

TODO!

- each `Antelope` type will have its own Rust struct definition
- these structs implement the `serialize`/`deserialize` traits from `serde`,
  this allows to (de)serialize them to JSON at least

# DESIGN DECISIONS

This part lists the design decisions that went into the library. It helps document
some decisions and their rationale and keep a trace so we don't have to ask the
same question or wonder why some choices have been made over and over again.

## own class for ByteStream

we investigated the possibility to use the `std::io::Read` and `std::io::Write`
trait but they don't provide enough convenience functions and don't bring much
to the table for us

we investigated the possibility to use the `bytes` crate which looks very nice,
except for one minor issue:
the read and write operation are both infallible. This is ok for write operations
for us, but for read operations that means that we panic if we reach the end of
the stream, which is something that we could expect and that we currently account
properly for with `StreamError`.

## Error handling

### `thiserror` vs `snafu`

we started with `thiserror` as it seems to be the most popular library for error
handling in the Rust ecosystem. It served us well to a point but it has a few
shortcomings for us:

- automatic error conversion using the #[from] attribute can only handle one
  instance of a specific source error for all variants
- errors lack some information such as location and/or backtrace which can make
  it hard to track their root causes easily

we switched to `snafu` for the following reasons:

- context selectors are very ergonomic (when one understands them!) and they
  allow to have some fields filled in automatically (location, backtrace)
- there is no automatic conversion from an error to another (easy to implement,
  though, see: `antelope_core::impl_auto_error_conversion` macro) and require
  to manually add a context every time we want to convert an error. This might
  seem overkill but is actually good practice: we get a full error stacktrace
  for every error, without "skipping" levels
- see these articles for inspiration:
  - <https://www.reddit.com/r/rust/comments/1cp8xtx/error_handling_for_large_rust_projects/>
  - <https://github.com/GreptimeTeam/greptimedb/blob/main/src/common/macro/src/stack_trace_debug.rs>
  - <https://www.reddit.com/r/rust/comments/dfs1zk/2019_q4_error_patterns_snafu_vs_errderive_anyhow/>
  - <https://dev.to/e_net4/migrating-from-quick-error-to-snafu-a-story-on-revamped-error-handling-in-rust-58h9>
  - <https://gist.github.com/quad/a8a7cc87d1401004c6a8973947f20365>

### Displaying errors: `color_eyre`

`color_eyre` is used to display nice reports, with backtraces etc.

It should only be used in unittests and user code, not in the libraries themselves.


## ABIProvider trait vs. ABIProvider enum

we started with ABIProvider being a trait to allow more flexibility and to allow
clients of the library to implement their own ABIProvider. This proved tricky
with respect to API and design (maybe due to our own inexperience), so we switched
to an enum representing all the possible ABIProvider implemented for now.
It should still be possible to extend this and implement new external ABIProviders
using a custom variant that takes ownership of a struct implementing a new
ABIProviderTrait or something similar.


# STYLE

## Import order

`use` directives go at the top of the file, in 3 groups (separated by a blank line):

- `std` imports
- other 3rd-party crates
- our own crate imports


## TODO: DECIDE: use `hex::encode_upper()` instead of `hex::encode()`

Antelope C++ code uses UPPER_CASE
Wharfkit TS code uses lower_case

it seems that lower case encoding is more readable as lower case letters have a
lower height than digits, whereas upper case letters have the same height

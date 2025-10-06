# ARCHITECTURE

TODO!

- each `Antelope` type will have its own Rust struct definition
- these structs implement the `serialize`/`deserialize` traits from `serde`,
  this allows to (de)serialize them to JSON at least

# DESIGN DECISIONS

This part lists the design decisions that went into the library. It helps document
some decisions and their rationale and keep a trace so we don't have to ask the
same question or wonder why some choices have been made over and over again.

## JSON use within the library

JSON serialization is handled by the `serde_json` crate, and serde traits are
derived for all our types.

Nonetheless, there are some differences in the way the Antelope C++ code
handles JSON:
 - `int64` and `int128` types are always quoted
 - `float32` and `float64` never use scientific notation

To get closer to that behavior, we implemented a JSON [Formatter]
-- [`kudu::json::VaultaFormatter`][crate::json::VaultaFormatter] -- to
properly output values in the format expected by Antelope which is automatically used
when calling [`kudu::json::to_string()`][crate::json::to_string()]

[Formatter]: https://docs.rs/serde_json/latest/serde_json/ser/trait.Formatter.html


## On the usage of `serde` for binary data

The library currently uses `serde` and its `Serialize` and `Deserialize` traits
in order to provide (de)serialization to JSON. We tried to use it also for
serialization to a binary stream (ie: ABI) and nearly achieved it, however we
ran into the following issues that made us decide to implement ABI serialization
using our own trait instead of `serde`:

- serialization of bytes slices goes through serialization of sequence of bytes
  (or tuple) and is quite inefficient because it will serialize each byte
  independently (by calling `serialize_seq` and `serialize_u8` for each byte).
  We tried repurposing the `serialize_bytes` method on the `Serializer` to write
  a byte slice **without** its length but that is problematic when deserializing
  as we don't know how many bytes we should be reading.
  See some related issues:
   - <https://github.com/serde-rs/serde/issues/2120>
   - <https://github.com/uuid-rs/uuid/issues/557>
   - <https://github.com/serde-rs/bytes>
   - serde-related crates: `serde_arrays`, `serde_with`
- serializing checksum types (ie: fixed-size arrays) would always serialize the
  length first, which we don't want, unless we use `serialize_bytes()` but it
  was already re-purposed (see previous point)
- deserializing variable-length fields (eg: `VarUint32`) proved impossible
- deserializing fixed-length arrays proved impossible too

On top of that, there were some *hacks* to try to serialize data to a binary stream
that made the rest of the code work but be sub-optimal.

The conclusion of this is that we use our own `ABISerializable` trait in
order to serialize structs to a binary stream.


## Own class for `ByteStream`

We investigated the possibility to use the `std::io::Read` and `std::io::Write`
trait but they don't provide enough convenience functions and don't bring much
to the table for us

We investigated the possibility to use the `bytes` crate which looks very nice,
except for one minor issue:
the read and write operation are both infallible. This is ok for write operations
for us (ie: we can always grow a vec or append to a file we are writing to), but
for read operations that means that we panic if we reach the end of the stream,
which is something that we could expect and that we currently account properly
for with `StreamError`.

### Open question on `ByteStream`

We considered the possibility of having a `ByteStream` trait with the following
methods:

```
pub trait ByteStream {
    fn read_byte(&mut self) -> Result<u8, StreamError>;
    fn read_bytes(&mut self, n: usize) -> Result<&[u8], StreamError>;

    fn write_byte(&mut self, byte: u8);
    fn write_bytes(&mut self, bytes: &[u8]);
}
```

and have a `DataStream` class implementing those, leaving open the possibility
for someone else to come with a better implementation that would fit this trait.

However, we ran into the following issues:

- `read_bytes()` returns a `&[u8]` with a lifetime bound to the one of the struct
  implementing the trait; this is not always desirable.
- `ABIDefinition::from_bin()` wants to call `leftover()` but that method is not
  part of the trait and does not belong in there.

So for now, `ByteStream` stays as a normal struct.

### Possible alternatives to `ByteStream`

- investigate the following as potential alternatives to the `ByteStream` struct:
  - <https://graphallthethings.com/posts/better-buf-read>
  - `std::io::Cursor`
  - something based on the `bytes` crate?
    - or this crate maybe: <https://github.com/wyfo/arc-slice> [[reddit](https://www.reddit.com/r/rust/comments/1j7sbwr/arcslice_a_generalized_implementation/)]
  - <https://docs.rs/binary-stream/>


## Error handling

### `thiserror` vs `snafu`

We started with `thiserror` as it seems to be the most popular library for error
handling in the Rust ecosystem. It served us well to a point but it has a few
shortcomings for us:

- automatic error conversion using the `#[from]` attribute can only handle one
  instance of a specific source error for all variants
- errors lack some information such as location and/or backtrace which can make
  it hard to track their root causes easily

We switched to `snafu` for the following reasons:

- context selectors are very ergonomic (when one understands them!) and they
  allow to have some fields filled in automatically (location, backtrace)
- there is no automatic conversion from an error to another (easy to implement,
  though, see: `kudu::impl_auto_error_conversion` macro) and require
  to manually add a context every time we want to convert an error. This might
  seem overkill but is actually good practice: we get a full error stacktrace
  for every error, without "skipping" levels
- see these articles for inspiration:
  - <https://www.reddit.com/r/rust/comments/1cp8xtx/error_handling_for_large_rust_projects/>
  - <https://github.com/GreptimeTeam/greptimedb/blob/main/src/common/macro/src/stack_trace_debug.rs>
  - <https://www.reddit.com/r/rust/comments/dfs1zk/2019_q4_error_patterns_snafu_vs_errderive_anyhow/>
  - <https://dev.to/e_net4/migrating-from-quick-error-to-snafu-a-story-on-revamped-error-handling-in-rust-58h9>
  - <https://gist.github.com/quad/a8a7cc87d1401004c6a8973947f20365>
  - <https://greptime.com/blogs/2024-05-07-error-rust> / <https://news.ycombinator.com/item?id=42457515>

### Displaying errors: `color_eyre`

`color_eyre` is used to display nice reports, with backtraces etc.

It should only be used in unittests and user code, not in the libraries themselves.


## ABIProvider trait vs. ABIProvider enum

We started with ABIProvider being a trait to allow more flexibility and to allow
clients of the library to implement their own ABIProvider. This proved tricky
with respect to API and design (maybe due to our own inexperience), so we switched
to an enum representing all the possible ABIProvider implemented for now.
It should still be possible to extend this and implement new external ABIProviders
using a custom variant that takes ownership of a struct implementing a new
ABIProviderTrait or something similar.


## Unsupported features

## `WebAuthn` signatures

TODO: not yet implemented


# STYLE GUIDE

## Import order

`use` directives go at the top of the file, in 3 groups (separated by a blank line):

- `std` imports
- other 3rd-party crates
- our own crate imports


## Error and logging messages style

- error and logging messages should be lower-case (ie: not be capitalized)

### Usage of single-quotes, double-quotes or backticks

- single quotes (`'`) for identifiers
- double quotes (`"`) for strings
- backticks (`` ` ``) for types

### Examples

- `` "expected type `int8` for field 'quantity' on struct 's1'" ``
- `` r#"value for 'obj.name' is "obj1" and its type is `str`"# ``


## Nomenclature

### `to_hex` / `to_bin`

`bin` means some binary data, usually `Bytes`/`Vec<u8>`/etc. while `hex` means an hexadecimal
representation of that data, usually `String`


## Usage of `unwrap`, `panic` and `assert`

Usage of `unwrap` in particular should be commented when we know the unwrapping is safe,
like so:

```
let v = vec![1, 2, 3];
if (v.len() > 0 &&
    v.get(0).unwrap() == 1)  // safe unwrap
{
    println!("yay!");
}
```

## Hex literals are written in lower-case

That also means that we should use `hex::encode()` instead of `hex::encode_upper()`

Antelope C++ code uses UPPER_CASE, while Wharfkit TypeScript code uses lower_case.

It seems that lower case encoding is more readable as lower case letters have a
lower height than digits, whereas upper case letters and digits all have the same
height so it's harder to distinguish groups of characters.

Most crypto websites (block explorers) seem to use lower case too.

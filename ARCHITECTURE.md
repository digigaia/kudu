# ARCHITECTURE

TODO

# DESIGN DECISIONS

## own class for ByteStream

we investigated the possibility to use the `std::io::Read` and `std::io::Write` trait but they
don't provide enough convenience functions and don't bring much to the table for us

we investigated the possibility to use the `bytes` crate which looks very nice, except for one minor
issue:
the read and write operation are both infallible. This is ok for write operations for us, but for
read operations that means that we panic if we reach the end of the stream, which is something that
we could expect and we currently account properly for it with `StreamError`.

# STYLE

- use `hex::encode_upper()` instead of `hex::encode()`

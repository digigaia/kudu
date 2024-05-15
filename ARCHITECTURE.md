# ARCHITECTURE

TODO

# DESIGN DECISIONS

## own class for ByteStream

we investigated the possibility to use the `std::io::Read` and `std::io::Write`
trait but they don't provide enough convenience functions and don't bring much
to the table for us

we investigated the possibility to use the `bytes` crate which looks very nice,
except for one minor issue:
the read and write operation are both infallible. This is ok for write operations
for us, but for read operations that means that we panic if we reach the end of
the stream, which is something that we could expect and we currently account
properly for it with `StreamError`.


## ABIProvider trait vs. ABIProvider enum

we started with ABIProvider being a trait to allow more flexibility and to allow
clients of the library to implement their own ABIProvider. This proved tricky
with respect to API and design (maybe due to our own inexperience), so we switched
to an enum representing all the possible ABIProvider implemented for now.
It should still be possible to extend this and implement new external ABIProviders
using a custom variant that takes ownership of a struct implementing a new
ABIProviderTrait or something similar.


# STYLE

- use `hex::encode_upper()` instead of `hex::encode()`

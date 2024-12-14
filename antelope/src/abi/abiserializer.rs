use std;
use std::fmt::{self, Display};

use serde::{ser, de, Serialize};
use tracing::{info, warn};

use crate::{BinarySerializable, ByteStream, VarUint32};

// =============================================================================
//
//     ABISerializer from Rust types/structs to Antelope binary stream
//
//     TODO:
//      - review Error type
//
// =============================================================================

// -----------------------------------------------------------------------------
//     Error type
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub enum Error {
    Message(String),
    Other,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::Other => formatter.write_str("unexpected error"),
        }
    }
}

impl std::error::Error for Error {}

type Result<T, E = Error> = core::result::Result<T, E>;


// -----------------------------------------------------------------------------
//     ABISerializer serde Serializer
// -----------------------------------------------------------------------------

pub struct ABISerializer {
    output: ByteStream,
}

// FIXME: can this fail? shouldn't we return Vec<u8> instead of Result<_>?
pub fn to_bin<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = ABISerializer {
        output: ByteStream::new(),
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output.into_bytes())
}

pub fn to_hex<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut serializer = ABISerializer {
        output: ByteStream::new(),
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output.hex_data())
}




impl<'a> ser::Serializer for &'a mut ABISerializer {
    // The output type produced by this `Serializer` during successful
    // serialization. Most serializers that produce text or binary output should
    // set `Ok = ()` and serialize into an `io::Write` or buffer contained
    // within the `Serializer` instance, as happens here. Serializers that build
    // in-memory data structures may be simplified by using `Ok` to propagate
    // the data structure around.
    type Ok = ();

    // The error type when some error occurs during serialization.
    type Error = Error;

    // Associated types for keeping track of additional state while serializing
    // compound data structures like sequences and maps. In this case no
    // additional state is required beyond what is already stored in the
    // Serializer struct.
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn is_human_readable(&self) -> bool {
        false
    }

    // -----------------------------------------------------------------------------
    //     Primitive types
    // -----------------------------------------------------------------------------

    fn serialize_bool(self, v: bool) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_i32(self, v:i32) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        if self.is_human_readable() {
            self.output.write_byte(b'"');
        }
        v.encode(&mut self.output);
        if self.is_human_readable() {
            self.output.write_byte(b'"');
        }
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        if self.is_human_readable() {
            self.output.write_byte(b'"');
        }
        v.encode(&mut self.output);
        if self.is_human_readable() {
            self.output.write_byte(b'"');
        }
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    fn serialize_char(self, _v: char) -> Result<()> {
        unimplemented!();
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        v.encode(&mut self.output);
        Ok(())
    }

    /// WARNING!!! This is to encode bytes as-is directly into the binary stream and
    /// doesn't contain the length of the bytes slice.
    /// To encode with the size it should be a `&Vec<u8>` instead of a `&[u8]`
    /// NOTE: actually this isn't called when serializing &[u8] so we should be safe
    ///       (serialize_seq is called instead)
    /// Checksum should use this, Bytes should use serialize_seq
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        warn!("serialize_bytes");
        self.output.write_bytes(v);
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        false.serialize(self)
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where T: ?Sized + Serialize
    {
        true.serialize(&mut *self)?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        unimplemented!();
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        unimplemented!();
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        unimplemented!();
    }

    fn serialize_newtype_struct<T>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<()>
    where T: ?Sized + Serialize {
        info!("serializing newtype struct: {name}");
        match name {
            "VarUint32" => {
                // let n = cast_ref::<T, u32>(value).unwrap();
                // todo!()
                panic!("haven't found yet how to properly serialize a naked VarUint32...");
                // value.serialize(self)
            },
            _ => {
                value.serialize(self)
            }
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where T: ?Sized + Serialize {
        unimplemented!();
    }

    fn serialize_seq(
        self,
        len: Option<usize>,
    ) -> Result<Self::SerializeSeq, Self::Error> {
        warn!("seq len: {:?}", len);
        let len = VarUint32(len.expect("Sequence must have a known length") as u32);  // FIXME: this as cast
        len.serialize(&mut *self)?;
        Ok(self)
    }

    fn serialize_tuple(
        self,
        _len: usize,
    ) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        unimplemented!();
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unimplemented!();
    }

    fn serialize_map(
        self,
        _len: Option<usize>,
    ) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!();
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        unimplemented!();
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        unimplemented!();
    }
}


// The following 7 impls deal with the serialization of compound types like
// sequences and maps. Serialization of such types is begun by a Serializer
// method and followed by zero or more calls to serialize individual elements of
// the compound type and one call to end the compound type.
//
// This impl is SerializeSeq so these methods are called after `serialize_seq`
// is called on the Serializer.
impl ser::SerializeSeq for &'_ mut ABISerializer {
    // Must match the `Ok` type of the serializer.
    type Ok = ();
    // Must match the `Error` type of the serializer.
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        warn!("serializing element from seq");
        value.serialize(&mut **self)
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        // self.output += "]";
        Ok(())
    }
}

// Same thing but for tuples.
impl ser::SerializeTuple for &'_ mut ABISerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // unimplemented!();
        // if !self.output.ends_with('[') {
        //     self.output += ",";
        // }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        // self.output += "]";
        Ok(())
    }
}

// Same thing but for tuple structs.
impl ser::SerializeTupleStruct for &'_ mut ABISerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Tuple variants are a little different. Refer back to the
// `serialize_tuple_variant` method above:
//
//    self.output += "{";
//    variant.serialize(&mut *self)?;
//    self.output += ":[";
//
// So the `end` method in this impl is responsible for closing both the `]` and
// the `}`.
impl ser::SerializeTupleVariant for &'_ mut ABISerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Some `Serialize` types are not able to hold a key and value in memory at the
// same time so `SerializeMap` implementations are required to support
// `serialize_key` and `serialize_value` individually.
//
// There is a third optional method on the `SerializeMap` trait. The
// `serialize_entry` method allows serializers to optimize for the case where
// key and value are both available simultaneously. In JSON it doesn't make a
// difference so the default behavior for `serialize_entry` is fine.
impl ser::SerializeMap for &'_ mut ABISerializer {
    type Ok = ();
    type Error = Error;

    // The Serde data model allows map keys to be any serializable type. JSON
    // only allows string keys so the implementation below will produce invalid
    // JSON if the key serializes as something other than a string.
    //
    // A real JSON serializer would need to validate that map keys are strings.
    // This can be done by using a different Serializer to serialize the key
    // (instead of `&mut **self`) and having that other serializer only
    // implement `serialize_str` and return an error on any other data type.
    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
        // if !self.output.ends_with('{') {
        //     self.output += ",";
        // }
        // key.serialize(&mut **self)
    }

    // It doesn't make a difference whether the colon is printed at the end of
    // `serialize_key` or at the beginning of `serialize_value`. In this case
    // the code is a bit simpler having it here.
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        // self.output += ":";
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        // self.output += "}";
        Ok(())
    }
}

// Structs are like maps in which the keys are constrained to be compile-time
// constant strings.
impl ser::SerializeStruct for &'_ mut ABISerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // key.serialize(&mut **self)?;
        // self.output += ":";
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

// Similar to `SerializeTupleVariant`, here the `end` method is responsible for
// closing both of the curly braces opened by `serialize_struct_variant`.
impl ser::SerializeStructVariant for &'_ mut ABISerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!();
        // key.serialize(&mut **self)?;
        // self.output += ":";
        // value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}


// =============================================================================
//
//     Unittests
//
// =============================================================================

#[cfg(test)]
mod tests {
    use std::sync::Once;
    use color_eyre::eyre::Result;
    use serde_json;
    use tracing_subscriber::{
        EnvFilter,
        // fmt::format::FmtSpan,
    };

    use crate::VarUint32;
    use super::*;

    static TRACING_INIT: Once = Once::new();

    fn init() {
        TRACING_INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter(EnvFilter::from_default_env())
            // .with_span_events(FmtSpan::ACTIVE)
                .init();
        });
    }

    #[track_caller]
    fn bin<T: Serialize>(value: T) -> String {
        to_hex(&value).unwrap()
    }

    #[test]
    fn primitive_types() -> Result<()> {
        init();

        assert_eq!(bin(false), "00");
        assert_eq!(bin(true), "01");

        assert_eq!(bin(1u8), "01");
        assert_eq!(bin(2u16), "0200");
        assert_eq!(bin(3u32), "03000000");
        assert_eq!(bin(4u64), "0400000000000000");

        assert_eq!(bin(170141183460469231731687303715884105727_i128), "ffffffffffffffffffffffffffffff7f");
        assert_eq!(bin(-170141183460469231731687303715884105728_i128), "00000000000000000000000000000080");
        assert_eq!(bin(0_u128), "00000000000000000000000000000000");
        assert_eq!(bin(18446744073709551615_u128), "ffffffffffffffff0000000000000000");
        assert_eq!(bin(340282366920938463463374607431768211454_u128), "feffffffffffffffffffffffffffffff");
        assert_eq!(bin(340282366920938463463374607431768211455_u128), "ffffffffffffffffffffffffffffffff");

        let s = String::from("This is a string.");
        assert_eq!(serde_json::to_string(&s)?, r#""This is a string.""#);
        assert_eq!(bin(s), "1154686973206973206120737472696e672e");

        Ok(())
    }

    #[test]
    fn test_array() -> Result<()> {
        init();

        let n = VarUint32(2300);
        assert_eq!(bin(n), "fc11");
        assert_eq!(serde_json::to_string(&n)?, "2300");

        let v = vec![1_u8, 2, 3];
        assert_eq!(bin(v.clone()), "03010203");

        let v2 = &v[..];
        assert_eq!(bin(v2), "03010203");

        #[derive(Serialize)]
        struct Bytes(Vec<u8>);

        let b = Bytes(vec![1_u8, 2, 3, 4]);
        assert_eq!(serde_json::to_string(&b)?, "[1,2,3,4]");
        assert_eq!(bin(b), "0401020304");

        Ok(())
    }

}

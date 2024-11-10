impl<T> ABISerialize<T> for T
where
    T: BinarySerializable
{
    fn to_bin(&self, stream: &mut ByteStream) {
        self.encode(stream)
    }
    fn from_bin(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Self::decode(stream)
    }

}

impl ABISerialize<VarInt32> for i32 {
    fn to_bin(&self, stream: &mut ByteStream) {
        VarInt32::from(*self).encode(stream)
    }
    fn from_bin(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(VarInt32::decode(stream)?.into())
    }
}

impl ABISerialize<VarUint32> for u32 {
    fn to_bin(&self, stream: &mut ByteStream) {
        VarUint32::from(*self).encode(stream)
    }
    fn from_bin(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        Ok(VarUint32::decode(stream)?.into())
    }
}


// TODO: more here, to enable optimizations, like
//
// impl ABISerialize<builtin::String> for &str
// ...

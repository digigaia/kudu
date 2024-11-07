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
        write_var_i32(stream, *self);
    }
    fn from_bin(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        read_var_i32(stream)
    }
}

impl ABISerialize<VarUint32> for u32 {
    fn to_bin(&self, stream: &mut ByteStream) {
        write_var_u32(stream, *self);
    }
    fn from_bin(stream: &mut ByteStream) -> Result<Self, SerializeError> {
        read_var_u32(stream)
    }
}


// TODO: more here, to enable optimizations, like
//
// impl ABISerialize<builtin::String> for &str
// ...

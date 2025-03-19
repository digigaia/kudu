use serde::{Deserialize, Deserializer, Serialize, Serializer};

// -----------------------------------------------------------------------------
//     VarInt32
// -----------------------------------------------------------------------------

/// Newtype wrapper around a `i32` that has a different serialization implementation
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct VarInt32(pub i32);

impl From<i32> for VarInt32 {
    fn from(n: i32) -> VarInt32 { VarInt32(n) }
}

impl From<VarInt32> for i32 {
    fn from(n: VarInt32) -> i32 { n.0 }
}

impl Serialize for VarInt32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VarInt32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let n = i32::deserialize(deserializer)?;
        Ok(n.into())
    }

}


// -----------------------------------------------------------------------------
//     VarUint32
// -----------------------------------------------------------------------------

/// Newtype wrapper around a `u32` that has a different serialization implementation
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct VarUint32(pub u32);

impl From<u32> for VarUint32 {
    fn from(n: u32) -> VarUint32 { VarUint32(n) }
}

impl From<VarUint32> for u32 {
    fn from(n: VarUint32) -> u32 { n.0 }
}

impl From<usize> for VarUint32 {
    fn from(n: usize) -> VarUint32 {
        let n: u32 = n.try_into().expect("number too large to fit in a `u32`");
        VarUint32(n)
    }
}

impl From<VarUint32> for usize {
    fn from(n: VarUint32) -> usize {
        n.0 as usize
    }
}

impl Serialize for VarUint32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VarUint32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let n = u32::deserialize(deserializer)?;
        Ok(n.into())
    }
}

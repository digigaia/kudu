use std::str::FromStr;

use snafu::ResultExt;

use crate::{
    JsonValue,
    convert::{ConversionError, HexDecodeSnafu, IncompatibleVariantTypesSnafu},
};

type Result<T, E = ConversionError> = std::result::Result<T, E>;

pub use float128::Float128;

#[cfg(not(feature = "float128"))]
mod float128 {
    use super::*;

    #[derive(Copy, Clone, Debug, PartialEq, Default)]
    pub struct Float128([u8;16]);

    impl Float128 {
        pub fn from_variant(v: &JsonValue) -> Result<Self> {
            variant_to_f128(v)
        }

        pub fn to_hex(&self) -> String {
            hex::encode(self.0)
        }

        pub fn to_bin_repr(&self) -> &[u8; 16] {
            &self.0
        }

        pub fn from_bin_repr(bin: &[u8; 16]) -> Self {
            Float128(*bin)
        }
    }

    pub fn variant_to_f128(v: &JsonValue) -> Result<Float128> {
        if let Some(x) = v.as_f64()      { Ok(x.into()) }
        else if let Some(s) = v.as_str() {
            let mut result = Float128::default();
            hex::decode_to_slice(s, &mut result.0).context(HexDecodeSnafu { repr: s })?;
            Ok(result)
        }
        else {
            IncompatibleVariantTypesSnafu { typename: "f128", value: v.clone() }.fail()
        }
    }

    impl From<f64> for Float128 {
        fn from(_value: f64) -> Self {
            unimplemented!("no native f128 support");
        }
    }

    impl FromStr for Float128 {
        type Err = ConversionError;

        fn from_str(_s: &str) -> Result<Self, Self::Err> {
            unimplemented!("no native f128 support");
        }
    }
}


#[cfg(feature = "float128")]
mod float128 {
    use super::*;

    use bytemuck::{cast_ref, pod_read_unaligned};

    use crate::convert::str_to_float;

    #[derive(Copy, Clone, Debug, PartialEq, Default)]
    pub struct Float128(f128);

    impl Float128 {
        pub fn from_variant(v: &JsonValue) -> Result<Self> {
            variant_to_f128(v).map(Float128)
        }

        pub fn to_hex(&self) -> String {
            hex::encode(self.0.to_ne_bytes())
        }

        pub fn to_bin_repr(&self) -> &[u8; 16] {
            cast_ref::<f128, [u8; 16]>(&self.0)
        }

        pub fn from_bin_repr(bin: &[u8; 16]) -> Self {
            Float128(pod_read_unaligned(bin))
        }
    }

    pub fn variant_to_f128(v: &JsonValue) -> Result<f128> {
        if let Some(x) = v.as_f64()      { Ok(x.into()) }
        else if let Some(s) = v.as_str() {
            let mut result = [0_u8; 16];
            hex::decode_to_slice(s, &mut result).context(HexDecodeSnafu { repr: s })?;
            Ok(f128::from_le_bytes(result))
        }
        else {
            IncompatibleVariantTypesSnafu { typename: "f128", value: v.clone() }.fail()
        }
    }

    impl From<f64> for Float128 {
        fn from(value: f64) -> Self {
            Float128(f128::from(value))
        }
    }

    impl FromStr for Float128 {
        type Err = ConversionError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            str_to_float::<f64>(s).map(|x| x.into())
        }
    }

}

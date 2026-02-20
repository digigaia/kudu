use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule(name = "crypto", submodule)]
pub mod kudu_crypto {
    use pyo3::prelude::*;

    use kudu::{ABISerializable, ByteStream, PrivateKey, PublicKey};

    use crate::util::{gen_bytes_conversion, value_err};

    // -----------------------------------------------------------------------------
    //     PrivateKey
    // -----------------------------------------------------------------------------

    #[pyclass(name = "PrivateKey", module = "kudu.crypto")]
    pub struct PyPrivateKey(pub PrivateKey);

    gen_bytes_conversion!("PyPrivateKey");

    #[pymethods]
    impl PyPrivateKey {
        #[new]
        fn new(key: &str) -> PyResult<Self> {
            Ok(Self(PrivateKey::new(key).map_err(value_err)?))
        }

        fn __str__(&self) -> String {
            format!("{}", &self.0)
        }

        fn __repr__(&self) -> String {
            format!("<kudu.PrivateKey: {}>", &self.0)
        }
    }


    // -----------------------------------------------------------------------------
    //     PublicKey
    // -----------------------------------------------------------------------------

    #[pyclass(name = "PublicKey", module = "kudu.crypto")]
    pub struct PyPublicKey(PublicKey);

    gen_bytes_conversion!("PyPublicKey");

    #[pymethods]
    impl PyPublicKey {
        #[new]
        fn new(key: &str) -> PyResult<Self> {
            Ok(Self(PublicKey::new(key).map_err(value_err)?))
        }

        fn __str__(&self) -> String {
            format!("{}", &self.0)
        }

        fn __repr__(&self) -> String {
            format!("<kudu.PublicKey: {}>", &self.0)
        }
    }
}

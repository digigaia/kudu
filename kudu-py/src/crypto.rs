// SPDX-FileCopyrightText: 2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule(name = "crypto", submodule)]
pub mod kudu_crypto {
    use pyo3::prelude::*;

    use kudu::{ABISerializable, PrivateKey, PublicKey};

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

        #[staticmethod]
        fn eosio_dev() -> Self {
            Self(PrivateKey::eosio_dev())
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

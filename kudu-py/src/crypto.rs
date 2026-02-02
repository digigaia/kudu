use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule(name = "crypto", submodule)]
pub mod kudu_crypto {
    // use std::string::ToString;

    use pyo3::prelude::*;
    // use pyo3::types::{PyDict, PyList, PyString};
    // use pythonize::{depythonize, pythonize};

    use kudu::{ABISerializable, ByteStream, PrivateKey, PublicKey};

    use crate::util::{
        gen_bytes_conversion,
        //gen_default_repr, gen_default_str, gen_dict_conversion, gen_string_getters,
        value_err
    };

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


        // // TODO: do we want to return a Bytes object or a str with decoded hex data?
        // #[getter]
        // fn get_data(&self) -> &[u8] {
        //     &self.0.data.0[..]
        // }

        // fn decode_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        //     Ok(pythonize(py, &self.0.decode_data().unwrap())?)
        // }

        // fn decode_data_with_abi<'py>(
        //     &self,
        //     py: Python<'py>,
        //     abi: &crate::abi::kudu_abi::PyABI
        // ) -> PyResult<Bound<'py, PyAny>> {
        //     Ok(pythonize(py, &self.0.decode_data_with_abi(&abi.0).map_err(value_err)?)?)
        // }
    }


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

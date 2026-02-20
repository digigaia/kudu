use pyo3::prelude::*;


/// A Python module implemented in Rust.
#[pymodule(name = "abi", submodule)]
pub mod kudu_abi {
    use pyo3::prelude::*;

    use kudu::abi::ABI;

    use crate::util::value_err;


    #[pyclass(name = "ABI", module = "kudu.abi")]
    pub struct PyABI(pub ABI);

    #[pymethods]
    impl PyABI {
        #[new]
        fn new(abi_definition: &str) -> PyResult<Self> {
            Ok(PyABI(ABI::from_str(abi_definition).map_err(value_err)?))
        }

        fn __repr__(&self) -> String {
            format!("<kudu.api.ABI: {:?}>", self.0)
        }

    }
}

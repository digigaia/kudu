use pyo3::prelude::*;

// TODO: investigate https://github.com/Jij-Inc/serde-pyobject, pros/cons vs pythonize?

/// A Python module implemented in Rust.
#[pymodule]
mod kudu {
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use pythonize::{depythonize, pythonize};
    use kudu::api::APIClient;
    use kudu::JsonValue;

    #[pyclass(name = "APIClient")]
    struct PyAPIClient(APIClient);

    #[pymethods]
    impl PyAPIClient {
        #[new]
        pub fn new(endpoint: &str) -> Self {
            PyAPIClient(APIClient::new(endpoint))
        }

        pub fn __repr__(&self) -> String {
            format!("<kudu.APIClient: {}>", self.0.endpoint)
        }

        pub fn __str__(&self) -> String {
            self.__repr__()
        }

        pub fn get<'py>(&self, py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyAny>> {
            let result = self.0.get(path);
            match result {
                Ok(v) => Ok(pythonize(py, &v)?),
                Err(e) => Err(PyValueError::new_err(format!("ureq error: {}", &e.to_string())))
            }
        }

        pub fn call<'py>(&self, py: Python<'py>, path: &str, params: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            let params: JsonValue = depythonize(params).unwrap();
            let result = self.0.call(path, &params);
            match result {
                Ok(v) => Ok(pythonize(py, &v)?),
                Err(e) => Err(PyValueError::new_err(format!("ureq error: {}", &e.to_string())))
            }
        }

    }

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        // Arbitrary code to run at the module initialization
        let args = ("https://jungle4.greymass.com",);
        m.add("jungle", m.getattr("APIClient")?.call1(args)?)?;
        Ok(())
    }
}

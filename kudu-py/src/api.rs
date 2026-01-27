use pyo3::prelude::*;


/// A Python module implemented in Rust.
#[pymodule(name = "api", submodule)]
pub mod kudu_api {
    use pyo3::exceptions::{PyRuntimeError, PyValueError};
    use pyo3::prelude::*;
    use pythonize::{depythonize, pythonize};
    use serde::Serialize;

    use kudu::api::{APIClient, HttpError};
    use kudu::JsonValue;

    fn wrap_for_python<'py, T>(py: Python<'py>, value: Result<&T, &HttpError>) -> PyResult<Bound<'py, PyAny>>
    where
        T: ?Sized + Serialize
    {
        match value {
            Ok(v) => Ok(pythonize(py, &v)?),
            Err(e) => Err(match e {
                HttpError::ConnectionError { source: _ } => PyRuntimeError::new_err(format!("http error: {}", e)),
                HttpError::JsonError { source: _ } => PyValueError::new_err(format!("json error: {}", e)),
            })
        }
    }

    #[pyclass(name = "APIClient", module = "kudu.api")]
    struct PyAPIClient(APIClient);

    #[pymethods]
    impl PyAPIClient {
        #[new]
        pub fn new(endpoint: &str) -> Self {
            PyAPIClient(APIClient::new(endpoint))
        }

        pub fn __repr__(&self) -> String {
            format!("<kudu.api.APIClient: {}>", self.0.endpoint)
        }

        // pub fn __str__(&self) -> String {
        //     self.__repr__()
        // }

        pub fn get<'py>(&self, py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyAny>> {
            let result = self.0.get(path);
            wrap_for_python(py, result.as_ref())
        }

        pub fn call<'py>(&self, py: Python<'py>, path: &str, params: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            let params: JsonValue = depythonize(params).unwrap();
            let result = self.0.call(path, &params);
            wrap_for_python(py, result.as_ref())
        }
    }
}

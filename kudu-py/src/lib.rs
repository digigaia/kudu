use pyo3::prelude::*;

// TODO: investigate https://github.com/Jij-Inc/serde-pyobject, pros/cons vs pythonize?

// TODO: investigate whether we want to use `benedict` as a replacement for barebones dicts

// NOTE: desired API for python bindings:
//
// def push_action(actor, contract, action, args, exception_type=None):
//     args = data_to_pyntelope(args)
//
//     logger.debug(f'==== ACTION: {actor} {contract} {action} {args}')
//
//     auth = pyntelope.Authorization(actor=actor, permission='active')
//     action = pyntelope.Action(account=contract, name=action, data=args, authorization=[auth])
//     signing_key = wallet.private_keys[actor]
//     result = pyntelope.Transaction(actions=[action]).link(net=net).sign(key=signing_key).send()


/// A Python module implemented in Rust.
#[pymodule]
mod kudu {
    use pyo3::exceptions::{PyRuntimeError, PyValueError};
    use pyo3::prelude::*;
    use pythonize::{depythonize, pythonize};
    use kudu::api::{APIClient, HttpError};
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
                Err(e) => Err(match e {
                    HttpError::ConnectionError { source: _ } => PyRuntimeError::new_err(format!("http error: {}", e)),
                    HttpError::JsonError { source: _ } => PyValueError::new_err(format!("json error: {}", e)),
                })
            }
        }

        pub fn call<'py>(&self, py: Python<'py>, path: &str, params: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            let params: JsonValue = depythonize(params).unwrap();
            let result = self.0.call(path, &params);
            match result {
                Ok(v) => Ok(pythonize(py, &v)?),
                Err(e) => Err(match e {
                    HttpError::ConnectionError { source: _ } => PyRuntimeError::new_err(format!("http error: {}", e)),
                    HttpError::JsonError { source: _ } => PyValueError::new_err(format!("json error: {}", e)),
                })
            }
        }
    }

    #[pymodule_export]
    use super::action::kudu_action;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        // Arbitrary code to run at the module initialization

        // properly declare submodules as packages
        // see: https://github.com/PyO3/pyo3/discussions/5397
        let modules = PyModule::import(m.py(), "sys")?.getattr("modules")?;
        modules.set_item("kudu.action", m.getattr("action")?)?;

        // create some useful global variables
        let args = ("http://127.0.0.1:8888",);
        m.add("local", m.getattr("APIClient")?.call1(args)?)?;

        let args = ("https://jungle4.greymass.com",);
        m.add("jungle", m.getattr("APIClient")?.call1(args)?)?;

        Ok(())
    }
}

mod action;

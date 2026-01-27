use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule(name = "chain", submodule)]
pub mod kudu_chain {
    use std::string::ToString;

    use pyo3::exceptions::{PyRuntimeError, PyValueError};
    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList};
    use pythonize::depythonize;

    use kudu::chain::{Action, IntoPermissionVec, PermissionLevel};
    use kudu::{ABISerializable, AccountName, ActionName, Bytes, ByteStream, PermissionName};

    #[inline]
    fn runtime_err<T: ToString>(e: T) -> PyErr {
        PyRuntimeError::new_err(e.to_string())
    }

    #[inline]
    fn value_err<T: ToString>(e: T) -> PyErr {
        PyValueError::new_err(e.to_string())
    }

    // -----------------------------------------------------------------------------
    //     PermissionLevel
    // -----------------------------------------------------------------------------

    #[pyclass(name = "PermissionLevel", module="kudu.chain")]
    struct PyPermissionLevel(PermissionLevel);

    #[pymethods]
    impl PyPermissionLevel {
        #[new]
        pub fn new(actor: &str, permission: &str) -> PyResult<Self> {
            let pl = PermissionLevel {
                actor: AccountName::new(actor).map_err(runtime_err)?,
                permission: PermissionName::new(permission).map_err(runtime_err)?,
            };
            Ok(Self(pl))
        }

        pub fn __repr__(&self) -> String {
            format!("<kudu.chain.PermissionLevel: {}@{}>", self.0.actor, self.0.permission)
        }

        pub fn __bytes__(&self) -> Vec<u8> {
            let mut b = ByteStream::new();
            self.0.to_bin(&mut b);
            b.into()
        }

        fn __eq__<'py>(&self, other: &Bound<'py, PyAny>) -> bool {
            // compare using a tuple (actor, permission)
            if let Ok((actor, permission)) = other.extract::<(&str, &str)>() {
                return self.0.actor == actor && self.0.permission == permission
            }
            // compare using a dict of named args
            let d: Result<&Bound<'py, PyDict>, _> = other.cast();
            if let Ok(d) = d {
                return d.len() == 2 && d.contains("actor").unwrap() && d.contains("permission").unwrap() && {
                    let actor: Result<String, _> = depythonize(&d.get_item("actor").unwrap().unwrap());
                    let permission: Result<String, _> = depythonize(&d.get_item("permission").unwrap().unwrap());
                    if let Ok(actor) = actor && let Ok(permission) = permission {
                        self.0.actor == actor && self.0.permission == permission

                    }
                    else { false }
                };
            }
            // compare using an object of the same type
            let p: Result<&Bound<'py, PyPermissionLevel>, _> = other.cast();
            if let Ok(p) = p {
                return self.0 == p.borrow().0;
            }
            false
        }

        #[getter]
        pub fn get_actor(&self) -> String {
            self.0.actor.to_string()
        }

        #[getter]
        pub fn get_permission(&self) -> String {
            self.0.permission.to_string()
        }

    }

    // -----------------------------------------------------------------------------
    //     Action
    // -----------------------------------------------------------------------------

    #[pyclass(name = "Action", module = "kudu.chain")]
    struct PyAction(Action);

    #[pymethods]
    impl PyAction {
        #[new]
        pub fn new(account: &str, name: &str, authorization: &PyPermissionLevel, data: &[u8]) -> PyResult<Self> {
            Ok(Self(Action {
                account: AccountName::new(account).map_err(value_err)?,
                name: ActionName::new(name).map_err(value_err)?,
                authorization: authorization.0.into_permission_vec(),
                data: Bytes(data.to_owned()),
            }))
        }

        pub fn __repr__(&self) -> String {
            format!("<kudu.Action: {}::{}({:?}) [auth: {:?}]>", self.0.account, self.0.name, self.0.data, self.0.authorization)
        }

        pub fn __bytes__(&self) -> Vec<u8> {
            let mut b = ByteStream::new();
            self.0.to_bin(&mut b);
            b.into()
        }

        #[getter]
        pub fn get_account(&self) -> String {
            self.0.account.to_string()
        }

        #[getter]
        pub fn get_name(&self) -> String {
            self.0.name.to_string()
        }

        #[getter]
        pub fn get_authorization<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
            let elements: Vec<PyPermissionLevel> = self.0.authorization.iter().map(|p| PyPermissionLevel(*p)).collect();
            let result = PyList::new(py, elements)?;
            Ok(result)
        }

        // #[getter]
        // pub fn get_authorization<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        //     Ok(pythonize(py, &self.0.authorization)?)
        // }

        #[getter]
        pub fn get_data(&self) -> &[u8] {
            &self.0.data.0[..]
        }


    }
}

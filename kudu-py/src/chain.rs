use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule(name = "chain", submodule)]
pub mod kudu_chain {
    use std::string::ToString;

    use pyo3::prelude::*;
    use pyo3::types::{PyDict, PyList};
    use pythonize::{depythonize, pythonize};

    use kudu::chain::{Action, IntoPermissionVec, PermissionLevel, Transaction};
    use kudu::{ABISerializable, AccountName, ActionName, Bytes, ByteStream, JsonValue, PermissionName};

    use crate::util::{
        gen_bytes_conversion, gen_default_repr, gen_default_str, gen_dict_conversion, gen_string_getters, runtime_err, value_err
    };

    // -----------------------------------------------------------------------------
    //     PermissionLevel
    // -----------------------------------------------------------------------------

    #[pyclass(name = "PermissionLevel", module="kudu.chain")]
    pub struct PyPermissionLevel(PermissionLevel);

    gen_default_repr!("PyPermissionLevel");
    gen_default_str!("PyPermissionLevel");
    gen_bytes_conversion!("PyPermissionLevel");
    gen_string_getters!("PyPermissionLevel", ["actor", "permission"]);

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

    }

    // -----------------------------------------------------------------------------
    //     Action
    // -----------------------------------------------------------------------------

    #[pyclass(name = "Action", module = "kudu.chain")]
    struct PyAction(Action);

    gen_bytes_conversion!("PyAction");
    gen_dict_conversion!("PyAction");
    gen_string_getters!("PyAction", ["account", "name"]);

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

        fn __eq__<'py>(&self, py: Python<'py>, other: &Bound<'py, PyAny>) -> bool {
            // NOTE: we do not use depythonize here because it doesn't work due to a borrowed string
            //       we might be able to fix this by looking at `Name::deserialize`
            //       instead, we convert our instance to a python dict and use that to compare

            // FIXME: we do not need to call `to_dict()` and `decoded()` in a row, we can detect
            //        which variant we need by checking `isinstance(other.data, str)`
            if other.is_instance_of::<PyDict>() {
                if let Ok(d) = self.to_dict(py) {
                    let result = d.call_method1("__eq__", (other,)).unwrap();
                    let same: bool = result.extract().expect("__eq__ needs to return a bool!");
                    if same { return true; }
                }

                if let Ok(d) = self.decoded(py) {
                    let result = d.call_method1("__eq__", (other,)).unwrap();
                    let same: bool = result.extract().expect("__eq__ needs to return a bool!");
                    if same { return true; }
                }
            }

            // compare using an object of the same type
            let p: Result<&Bound<'py, PyAction>, _> = other.cast();
            if let Ok(p) = p {
                return self.0 == p.borrow().0;
            }

            false
        }

        // pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        //     Ok(pythonize(py, &self.0)?)
        // }

        pub fn decoded<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            Ok(pythonize(py, &self.0.to_json().map_err(value_err)?)?)
        }


        #[getter]
        pub fn get_authorization<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
            let elements: Vec<PyPermissionLevel> = self.0.authorization.iter().map(|p| PyPermissionLevel(*p)).collect();
            let result = PyList::new(py, elements)?;
            Ok(result)
        }

        #[getter]
        pub fn get_data(&self) -> &[u8] {
            &self.0.data.0[..]
        }

        pub fn decode_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            Ok(pythonize(py, &self.0.decode_data().unwrap())?)
        }

        pub fn decode_data_with_abi<'py>(
            &self,
            py: Python<'py>,
            abi: &crate::abi::kudu_abi::PyABI
        ) -> PyResult<Bound<'py, PyAny>> {
            Ok(pythonize(py, &self.0.decode_data_with_abi(&abi.0).unwrap())?)  // FIXME: remove unwrap!!
        }

    }

    // -----------------------------------------------------------------------------
    //     Transaction
    // -----------------------------------------------------------------------------

    #[pyclass(name = "Transaction", module = "kudu.chain")]
    struct PyTransaction(Transaction);

    gen_bytes_conversion!("PyTransaction");
    gen_dict_conversion!("PyTransaction");

    #[pymethods]
    impl PyTransaction {
        #[new]
        pub fn new<'py>(tx: &Bound<'py, PyAny>) -> PyResult<Self> {
            let json: JsonValue = depythonize(tx)?;
            Ok(Self(Transaction::from_json(&json).unwrap()))  // FIXME: unwrap
        }

        pub fn __repr__(&self) -> String {
            format!("<kudu.Transaction: {:?}>", self.0.actions)
        }

        #[getter]
        pub fn get_ref_block_num(&self) -> u16 {
            self.0.ref_block_num
        }

        #[getter]
        pub fn get_actions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
            let elements: Vec<PyAction> = self.0.actions.iter().map(|a| PyAction(a.clone())).collect();
            PyList::new(py, elements)
        }

    }
}

use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule(name = "chain", submodule)]
pub mod kudu_chain {
    use std::string::ToString;

    use pyo3::prelude::*;
    use pyo3::types::{PyBytes, PyDict, PyList, PyString};
    use pythonize::{depythonize, pythonize};

    use kudu::chain::{Action, PermissionLevel, SignedTransaction, Transaction};
    use kudu::{ABISerializable, AccountName, ActionName, Bytes, ByteStream, JsonValue, PermissionName};

    use crate::api::kudu_api::PyAPIClient;
    use crate::crypto::kudu_crypto::PyPrivateKey;
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
        fn new(actor: &str, permission: &str) -> PyResult<Self> {
            let pl = PermissionLevel {
                actor: AccountName::new(actor).map_err(value_err)?,
                permission: PermissionName::new(permission).map_err(value_err)?,
            };
            Ok(Self(pl))
        }

        #[staticmethod]
        fn from_py<'py>(other: &Bound<'py, PyAny>) -> PyResult<Self> {
            // other object is of the same type
            let perm: Result<&Bound<'py, PyPermissionLevel>, _> = other.cast();
            if let Ok(perm) = perm {
                return Ok(Self(perm.borrow().0))
            }
            // other object is a tuple (actor, permission)
            if let Ok((actor, permission)) = other.extract::<(&str, &str)>() {
                return Self::new(actor, permission);
            }

            Err(value_err(format!("Cannot create PermissionLevel from object: {} [{}]", other, other.get_type())))
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
        fn new<'py>(
            account: &str,
            name: &str,
            authorization: &Bound<'py, PyAny>,
            data: &Bound<'py, PyAny>
        ) -> PyResult<Self> {
            // parse authorization param: can be a single PermissionLevel, a list of them, or python equivalents thereof
            let mut auth: Vec<PermissionLevel> = vec![];
            if let Ok(perm) = PyPermissionLevel::from_py(authorization) {
                auth.push(perm.0);
            }
            else if let Ok(authorization) = authorization.cast::<PyList>() {
                for value in authorization {
                    auth.push(PyPermissionLevel::from_py(&value)?.0);
                }
            }
            else {
                return Err(value_err(format!("invalid value for PermissionLevel: {}", 23)));
            }

            let action = if let Ok(data) = data.cast::<PyBytes>() {
                Action {
                    account: AccountName::new(account).map_err(value_err)?,
                    name: ActionName::new(name).map_err(value_err)?,
                    authorization: auth,
                    data: data.as_bytes().into(),
                }
            }
            else {
                let args: JsonValue = depythonize(data)?;

                Action {
                    account: AccountName::new(account).map_err(value_err)?,
                    name: ActionName::new(name).map_err(value_err)?,
                    authorization: auth,
                    data: Bytes::new(),
                }
                .with_data(&args)
            };

            Ok(Self(action))
        }

        fn __repr__(&self) -> String {
            format!("<kudu.Action: {}::{}(...) {:?}>", self.0.account, self.0.name, self.0.authorization)
        }

        fn __eq__<'py>(&self, py: Python<'py>, other: &Bound<'py, PyAny>) -> bool {
            // NOTE: we do not use depythonize here because it doesn't work due to a borrowed string
            //       we might be able to fix this by looking at `Name::deserialize`
            //       instead, we convert our instance to a python dict and use that to compare
            if other.is_instance_of::<PyDict>() {
                if other.get_item("data").unwrap().is_instance_of::<PyString>() {  // safe unwrap
                    // `.data` is an hex representation of the encoded action data
                    if let Ok(d) = self.to_dict(py) {
                        let result = d.call_method1("__eq__", (other,)).unwrap();
                        let same: bool = result.extract().expect("__eq__ needs to return a bool!");
                        if same { return true; }
                    }
                }
                else {
                    // `.data` is the decoded action data
                    if let Ok(d) = self.decoded(py) {
                        let result = d.call_method1("__eq__", (other,)).unwrap();
                        let same: bool = result.extract().expect("__eq__ needs to return a bool!");
                        if same { return true; }
                    }
                }
            }

            // compare using an object of the same type
            let p: Result<&Bound<'py, PyAction>, _> = other.cast();
            if let Ok(p) = p {
                return self.0 == p.borrow().0;
            }

            false
        }

        fn decoded<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            Ok(pythonize(py, &self.0.to_json().map_err(value_err)?)?)
        }

        #[getter]
        fn get_authorization<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
            let elements: Vec<PyPermissionLevel> = self.0.authorization.iter().map(|p| PyPermissionLevel(*p)).collect();
            let result = PyList::new(py, elements)?;
            Ok(result)
        }

        // TODO: do we want to return a Bytes object or a str with decoded hex data?
        #[getter]
        fn get_data(&self) -> &[u8] {
            &self.0.data.0[..]
        }

        fn decode_data<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            Ok(pythonize(py, &self.0.decode_data().unwrap())?)
        }

        fn decode_data_with_abi<'py>(
            &self,
            py: Python<'py>,
            abi: &crate::abi::kudu_abi::PyABI
        ) -> PyResult<Bound<'py, PyAny>> {
            Ok(pythonize(py, &self.0.decode_data_with_abi(&abi.0).map_err(value_err)?)?)
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
        fn new<'py>(tx: &Bound<'py, PyAny>) -> PyResult<Self> {
            // TODO: allow to use a PyAction as input
            let json: JsonValue = depythonize(tx)?;
            Ok(Self(Transaction::from_json(&json).map_err(value_err)?))
        }

        fn __repr__(&self) -> String {
            format!("<kudu.Transaction: {:?}>", self.0.actions)
        }

        #[getter]
        fn get_ref_block_num(&self) -> u16 {
            self.0.ref_block_num
        }

        #[getter]
        fn get_actions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
            let elements: Vec<PyAction> = self.0.actions.iter().map(|a| PyAction(a.clone())).collect();
            PyList::new(py, elements)
        }

        fn link(&mut self, client: &PyAPIClient) -> PyResult<()> {
            self.0.link(client.0.clone()).map_err(runtime_err)?;
            Ok(())
        }

        fn sign<'py>(&self, py: Python<'py>, key: &PyPrivateKey) -> PyResult<Bound<'py, PySignedTransaction>> {
            let result = PySignedTransaction(self.0.sign(&key.0).map_err(value_err)?);
            Bound::new(py, result)
        }
    }

    // -----------------------------------------------------------------------------
    //     SignedTransaction
    // -----------------------------------------------------------------------------

    #[pyclass(name = "SignedTransaction", module = "kudu.chain")]
    struct PySignedTransaction(pub SignedTransaction);

    // gen_bytes_conversion!("PySignedTransaction");
    gen_dict_conversion!("PySignedTransaction");

    #[pymethods]
    impl PySignedTransaction {
        fn __repr__(&self) -> String {
            format!("<kudu.SignedTransaction: {:?}>", self.0.tx)
        }

        fn send<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            let trace = self.0.send().map_err(runtime_err)?;
            Ok(pythonize(py, &trace)?)
        }

        // fn to_json(&self) -> JsonValue {
        //     self.0.to_json()
        // }
    }

}

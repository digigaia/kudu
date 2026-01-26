use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule(name = "chain", submodule)]
pub mod kudu_chain {
    use pyo3::exceptions::PyRuntimeError;
    use pyo3::prelude::*;
    use kudu::chain::{Action, PermissionLevel};
    use kudu::{AccountName, JsonValue, PermissionName};

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
                actor: AccountName::new(actor).map_err(|e| PyRuntimeError::new_err(e.to_string()))?,
                permission: PermissionName::new(permission).map_err(|e| PyRuntimeError::new_err(e.to_string()))?,
            };
            Ok(Self(pl))
        }

        pub fn __repr__(&self) -> String {
            format!("<kudu.chain.PermissionLevel: {}@{}>", self.0.actor, self.0.permission)
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

    #[pyclass(name = "Action", module="kudu.chain")]
    struct PyAction(Action);

    #[pymethods]
    impl PyAction {
        #[new]
        pub fn new() -> Self {
            todo!()
        }

        pub fn __repr__(&self) -> String {
            let action_args = "...";
            format!("<kudu.Action: {}>", action_args)
        }

        pub fn __str__(&self) -> String {
            self.__repr__()
        }
    }
}

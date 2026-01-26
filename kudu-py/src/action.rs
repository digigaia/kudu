use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule(name = "action", submodule)]
pub mod kudu_action {
    use pyo3::prelude::*;
    use kudu::chain::Action;
    use kudu::JsonValue;

    #[pyclass(name = "Action", module="kudu.action")]
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

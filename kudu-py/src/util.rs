use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};

#[inline]
pub fn value_err<T: ToString>(e: T) -> PyErr {
    PyValueError::new_err(e.to_string())
}

#[inline]
pub fn runtime_err<T: ToString>(e: T) -> PyErr {
    PyRuntimeError::new_err(e.to_string())
}

#[crabtime::function]
fn _gen_default_repr(struct_name: String) {
    crabtime::output! {
        use pyo3::PyTypeInfo;

        #[pymethods]
        impl {{struct_name}} {
            pub fn __repr__(&self) -> String {
                format!("<{}.{}: {}>", Self::MODULE.unwrap_or("unknown"), Self::NAME, self.0)
            }
        }
    }
}

#[crabtime::function]
fn _gen_default_str(struct_name: String) {
    crabtime::output! {
        #[pymethods]
        impl {{struct_name}} {
            pub fn __str__(&self) -> String {
                self.0.to_string()
            }
        }
    }
}

#[crabtime::function]
fn _gen_bytes_conversion(struct_name: String) {
    crabtime::output! {
        #[pymethods]
        impl {{struct_name}} {
            pub fn __bytes__(&self) -> Vec<u8> {
                let mut b = ByteStream::new();
                self.0.to_bin(&mut b);
                b.into()
            }
        }
    }
}

#[crabtime::function]
fn _gen_dict_conversion(struct_name: String) {
    crabtime::output! {
        #[pymethods]
        impl {{struct_name}} {
            pub fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
                Ok(pythonize(py, &self.0)?)
            }
        }
    }
}

#[crabtime::function]
fn _gen_string_getters(struct_name: String, vars: Vec<String>) {
    for var in vars {
        crabtime::output! {
            #[pymethods]
            impl {{struct_name}} {
                #[getter]
                pub fn get_{{var}}(&self) -> String {
                    self.0.{{var}}.to_string()
                }
            }
        }
    }
}


// NOTE: this is needed so we can access our macro from elsewhere in the crate
//       see: https://github.com/rust-lang/rust/pull/52234
pub(crate) use _gen_default_repr as gen_default_repr;
pub(crate) use _gen_default_str as gen_default_str;
pub(crate) use _gen_bytes_conversion as gen_bytes_conversion;
pub(crate) use _gen_dict_conversion as gen_dict_conversion;
pub(crate) use _gen_string_getters as gen_string_getters;

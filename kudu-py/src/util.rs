// SPDX-FileCopyrightText: 2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::error::Error;

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};

fn full_error_message<T: Error>(e: T) -> String {
    let mut message = vec![e.to_string()];
    let mut current_source = e.source();
    while let Some(source) = current_source {
        message.push(source.to_string());
        current_source = source.source();
    }
    message.join(": ")
}

#[inline]
pub fn value_err<T: Error>(e: T) -> PyErr {
    PyValueError::new_err(full_error_message(e))
}

#[inline]
pub fn runtime_err<T: Error>(e: T) -> PyErr {
    PyRuntimeError::new_err(full_error_message(e))
}

#[crabtime::function]
fn _gen_default_repr(struct_name: String) {
    crabtime::output! {
        #[pymethods]
        impl {{struct_name}} {
            pub fn __repr__<'py>(&self, py: Python<'py>) -> PyResult<String> {
                let pytype = <Self as ::pyo3::PyTypeInfo>::type_object(py);
                let pymodule = pytype.module()?;
                let module = pymodule.to_str()?;
                let classname = pytype.name()?;
                // only keep the root module as we import all in it anyway
                let module = match module.find('.') {
                    Some(idx) => &module[..idx],
                    None => module,
                };
                Ok(format!("<{}.{}: {}>", module, classname.to_str()?, self.0))
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
                let mut b = ::kudu::Bytes::new();
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

#[crabtime::function]
fn _gen_int_getters(struct_name: String, int_type: String, vars: Vec<String>) {
    for var in vars {
        crabtime::output! {
            #[pymethods]
            impl {{struct_name}} {
                #[getter]
                pub fn get_{{var}}(&self) -> {{int_type}} {
                    // use `.into()` to ensure we cover all infallible int conversions
                    self.0.{{var}}.into()
                }
            }
        }
    }
}

#[crabtime::function]
fn _gen_convert_getters(struct_name: String, convert: String, result_type: String, vars: Vec<String>) {
    for var in vars {
        crabtime::output! {
            #[pymethods]
            impl {{struct_name}} {
                #[getter]
                pub fn get_{{var}}(&self) -> {{result_type}} {
                    // use `.into()` to ensure we cover all infallible conversions
                    self.0{{convert}}.{{var}}().into()
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
pub(crate) use _gen_int_getters as gen_int_getters;
pub(crate) use _gen_convert_getters as gen_convert_getters;

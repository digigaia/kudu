use pyo3::prelude::*;

// TODO: investigate https://github.com/Jij-Inc/serde-pyobject, pros/cons vs pythonize?

// TODO: investigate whether we want to use `benedict` as a replacement for barebones dicts

// TODO: remove `pub` on pymethods, they do not need to be public

// TODO: define macro to easily declare getters for the wrapped types

// TODO: have the __bytes__() method be declared automatically

// TODO: implement the Transaction::to_dict() method for all wrapped classes. Maybe call it to_python

// TODO: factor runtime_err, value_err

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
    use pyo3::prelude::*;
    use pyo3::exceptions::PyValueError;
    use kudu::{ABISerializable, ByteStream, Name};

    #[pymodule_export]
    use super::abi::kudu_abi;

    #[pymodule_export]
    use super::api::kudu_api;

    #[pymodule_export]
    use super::chain::kudu_chain;

    #[inline]
    fn value_err<T: ToString>(e: T) -> PyErr {
        PyValueError::new_err(e.to_string())
    }

    #[pyclass(name = "Name")]
    struct PyName(Name);

    #[pymethods]
    impl PyName {
        #[new]
        pub fn new(name: &str) -> PyResult<Self> {
            Ok(Self(Name::new(name).map_err(value_err)?))
        }

        pub fn __repr__(&self) -> String {
            format!("'{}'", self.0)
        }

        pub fn __str__(&self) -> String {
            self.0.to_string()
        }

        pub fn __bytes__(&self) -> Vec<u8> {
            let mut b = ByteStream::new();
            self.0.to_bin(&mut b);
            b.into()
        }

        fn __eq__<'py>(&self, other: &Bound<'py, PyAny>) -> bool {
            // compare using a python string
            if let Ok(name) = other.extract::<&str>() {
                return self.0 == name
            }
            // compare using an object of the same type
            let p: Result<&Bound<'py, PyName>, _> = other.cast();
            if let Ok(p) = p {
                return self.0 == p.borrow().0;
            }
            false
        }
    }


    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        // Arbitrary code to run at the module initialization
        // There is also some further python code that is run, it can be
        // found in `../python/kudu/__init__.py`

        // properly declare submodules as packages
        // see: https://github.com/PyO3/pyo3/discussions/5397
        let modules = PyModule::import(m.py(), "sys")?.getattr("modules")?;
        modules.set_item("kudu.abi", m.getattr("abi")?)?;
        modules.set_item("kudu.api", m.getattr("api")?)?;
        modules.set_item("kudu.chain", m.getattr("chain")?)?;

        // create some useful global variables
        let api_client = m.getattr("api")?.getattr("APIClient")?;
        m.add("local", api_client.call1(("http://127.0.0.1:8888",))?)?;
        m.add("vaulta", api_client.call1(("https://api.eos.detroitledger.tech",))?)?;
        m.add("jungle", api_client.call1(("https://jungle4.greymass.com",))?)?;

        Ok(())
    }
}

mod abi;
mod api;
mod chain;

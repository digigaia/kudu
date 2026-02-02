use pyo3::prelude::*;

mod abi;
mod api;
mod chain;
mod crypto;
mod util;

// TODO: investigate https://github.com/Jij-Inc/serde-pyobject, pros/cons vs pythonize?

// TODO: investigate whether we want to use `benedict` as a replacement for barebones dicts

// TODO: investigate eyre for error reporting instead of our value_err, runtime_err wrappers
//       see: https://pyo3.rs/main/doc/pyo3/eyre/

// FIXME: check for abusive unwrap usage

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
    use kudu::{ABISerializable, ByteStream, Name};

    #[pymodule_export]
    use crate::abi::kudu_abi;

    #[pymodule_export]
    use crate::api::kudu_api;

    #[pymodule_export]
    use crate::chain::kudu_chain;

    #[pymodule_export]
    use crate::crypto::kudu_crypto;

    use crate::util::{gen_default_repr, gen_default_str, gen_bytes_conversion, value_err};

    // -----------------------------------------------------------------------------
    //     Name
    // -----------------------------------------------------------------------------

    #[pyclass(name = "Name")]
    struct PyName(Name);

    gen_default_repr!("PyName");
    gen_default_str!("PyName");
    gen_bytes_conversion!("PyName");

    #[pymethods]
    impl PyName {
        #[new]
        fn new(name: &str) -> PyResult<Self> {
            Ok(Self(Name::new(name).map_err(value_err)?))
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


    // -----------------------------------------------------------------------------
    //     Module initialization
    // -----------------------------------------------------------------------------

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
        modules.set_item("kudu.crypto", m.getattr("crypto")?)?;

        // create some useful global variables
        let api_client = m.getattr("api")?.getattr("APIClient")?;
        m.add("local", api_client.call1(("http://127.0.0.1:8888",))?)?;
        // m.add("vaulta", api_client.call1(("https://api.eos.detroitledger.tech",))?)?;
        m.add("vaulta", api_client.call1(("https://vaulta.greymass.com",))?)?;
        m.add("jungle", api_client.call1(("https://jungle4.greymass.com",))?)?;

        Ok(())
    }
}

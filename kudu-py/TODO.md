# TODO / FIXME

- folder needs to have a README.md file

- python version in `kudu-py/pyproject.toml` and `kudu-py/Cargo.toml` need to be updated by `just set-version`

- use a new type for LinkedTransaction, to avoid some runtime checks

- Readme needs to mention python bindings

- replace "let perm: Result<&Bound<'py, PyPermissionLevel>, _> = other.cast();" with "if let Some(perm) = other.cast::<PyPermissionLevel>()" everywhere and for all types

- push_action in test_chain.py needs to be implemented in Rust, with python bindings

- wallet.py needs to be implemented in Rust, with python bindings

- integration between tracing and python logging: https://pyo3.rs/v0.28.2/ecosystem/tracing

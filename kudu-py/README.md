<!--
SPDX-FileCopyrightText: 2026 DigiGaia SCCL
SPDX-License-Identifier: AGPL-3.0-or-later
-->

# Kudu-py

**A Python library for interacting with Antelope blockchains**

[![Latest published Kudu-py version](https://img.shields.io/pypi/v/kudu-py.svg)](https://pypi.org/project/kudu-py/)
[![AGPLv3+ license](https://img.shields.io/pypi/l/kudu)](https://github.com/digigaia/kudu/blob/master/LICENSES/AGPL-3.0-or-later.txt)

</div>

Kudu is a Rust library that provides data types and functions to interact with
[Antelope](https://antelope.io) blockchains.

Kudu-py is a library of python bindings for the Kudu library.

## Installing

```
uv pip install kudu-py
```

## Compiling instructions

To build and install in a local venv, make sure you have `maturin` installed (e.g.: `uv tool install maturin`)
and run the following:

```sh
uv sync  # just once, will create the venv
maturin develop
```

you can then run the tests:

```sh
uv run pytest
```

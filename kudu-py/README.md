<!--
SPDX-FileCopyrightText: 2026 DigiGaia SCCL
SPDX-License-Identifier: AGPL-3.0-or-later
-->

# README

This folder contains the implementation of python bindings for the Kudu library.


## Compiling instructions

To build and install in a local venv, run the following:

```sh
uv sync  # just once, will create the venv
uv run maturin develop
```

you can then run the tests:

```sh
uv run pytest
```

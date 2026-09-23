<!--
SPDX-FileCopyrightText: 2026 DigiGaia SCCL
SPDX-License-Identifier: AGPL-3.0-or-later
-->

This file contains notes about developing on Kudu.

# Release process

## Prerequisites

- ensure you have `cargo-edit` installed (needed to set version automatically)


## Steps to release a new version

In order to release a new version, you need to follow these steps:

- ensure you are on the `master` branch
- create a new commit with:
  - run `just license-all` to update license for all files
  - run `just set-version <version>`
  - update changelog
- tag this commit as `v<version>` (ie: leading 'v' for 'version')
- `git push --tags`

The CI running on Github Actions will automatically build and publish cargo
crates and python wheels.


## TODO / FIXME

- move `just license-all` into `just set-version` or even better into a `prek` rule.
  (note: if in `prek`, we should just `reuse lint` instead of a full blown `just license-all`)

- maybe move `just gen-gha-workflows` in `prek` if we have it instead of it being only
  in `just set-version`

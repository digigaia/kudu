<!--
SPDX-FileCopyrightText: 2026 DigiGaia SCCL
SPDX-License-Identifier: AGPL-3.0-or-later
-->

This file contains notes about developing on Kudu.

# Release process

In order to release a new version, you need to follow these steps:

- ensure you are on the `master` branch
- create a new commit with:
  - run `just set-version <version>`
  - update changelog
  - run `cargo update --workspace` to update lock file (TODO: move this into `just set-version`)
  - run `just license-all` to update license for all files (TODO: move this into `just set-version`)
  - run `just gen-gha-workflows` (TODO: move this into `just set-version`)
- tag this commit as `v<version>` (ie: leading 'v' for 'version')
- `git push --tags`

The CI running on Github Actions will automatically build and publish cargo
crates and python wheels.

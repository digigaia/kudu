// SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

package CI

name: "tests"

on: {
    push: branches: ["master"],
    pull_request: branches: ["master"],
}

permissions:
    contents: "read"

jobs: {
    "test-rust": uses: "./.github/workflows/tests-rust.yml",

	// deactivate those on normal pushes and PRs
	// we will only run them in the `release` workflow before publishing wheels
	// "test-python": {
	// 	needs: "test-rust"
	// 	uses: "./.github/workflows/tests-python.yml"
	// },
}

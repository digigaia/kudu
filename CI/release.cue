// SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

package CI

name: "release"

on: push: tags: ["*"]

permissions:
    contents: "read"

jobs: {
    "test-rust": uses: "./.github/workflows/tests-rust.yml",

	"test-python": {
		needs: "test-rust"
		uses: "./.github/workflows/tests-python.yml"
	},

	"release-rust": {
		needs: "test-rust"
		uses: "./.github/workflows/release-rust.yml"
	},

	"release-python": {
		needs: "test-python"
		uses: "./.github/workflows/release-python.yml"
	},
}

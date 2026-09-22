// SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

package CI

name: "release"

on: push: tags: ["*"]

permissions: {
	contents: "read"
	// contents: "write",      // used to upload release artifacts  // NOT USED FOR NOW BUT WILL BE
	"id-token": "write",    // used to sign the release artifacts
	attestations: "write",  // used to generate artifact attestation
}

jobs: {
    "test-rust": uses: "./.github/workflows/tests-rust.yml",

	"test-python": {
		needs: "test-rust"
		uses: "./.github/workflows/tests-python.yml"
	},

	"release-rust": {
		needs: "test-rust"
		uses: "./.github/workflows/release-rust.yml"
		secrets: "inherit"
	},

	"release-python": {
		needs: "test-python"
		uses: "./.github/workflows/release-python.yml"
	},
}

// SPDX-FileCopyrightText: 2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

package CI

name: "tests"

on: {
	push: branches: ["master"],
	pull_request: {
		branches: ["master"],
		types: ["opened", "synchronize", "reopened", "ready_for_review"],
	},
	workflow_dispatch: null,  // allow to call this workflow manually
}

permissions:
    contents: "read"

concurrency: {
	group: "${{ github.workflow }}-${{ github.ref }}"
	"cancel-in-progress": "${{ github.event_name == 'pull_request' }}"
}

jobs: {
    "test-rust": uses: "./.github/workflows/tests-rust.yml",

	// deactivate those on normal pushes and PRs in draft mode
	// can still trigger them manually
	"test-python": {
		needs: "test-rust"
		if: "github.event_name == 'workflow_dispatch' || github.event.pull_request.draft == false"
		uses: "./.github/workflows/tests-python.yml"
	},
}

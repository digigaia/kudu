// SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

package CI

name: "tests-rust"

// allow other workflows to call this one
on: workflow_call: null

permissions:
    contents: "read"

// define "runs-on" and a timeout for all jobs
jobs: [_]: #job

jobs: {
    "test-rust": {
		"timeout-minutes": 3,
		strategy: matrix: platform: [
			#ubuntu_runner,
			#ubuntu_2604_arm_runner,
			#macos_runner,
	    ],
        steps: [
			{ uses: #checkout },
			{ uses: #install_action,
			  with: tool: "just,cargo-nextest" },
			{ uses: #rust_cache },
			{ name: "Run tests", run: "just test" },
		],
	},
}

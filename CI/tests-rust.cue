// SPDX-FileCopyrightText: 2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

package CI

name: "tests-rust"

on: workflow_call: null      // allow other workflows to call this one
on: workflow_dispatch: null  // allow to call this workflow manually

permissions:
    contents: "read"

// inherit some default properties for all our jobs
jobs: [_]: #job

jobs: {
    "unittests": {
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

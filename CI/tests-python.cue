// SPDX-FileCopyrightText: 2024-2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

package CI

name: "tests-python"

// allow other workflows to call this one
on: workflow_call: null

permissions:
    contents: "read"

jobs: {
	"test-python": #job & {
		// do not run on `aarch64`, as the EOS VM doesn't run on it
		strategy: matrix: platform: [#ubuntu_runner],
        steps: [
			{ uses: #checkout },
			{ uses: #install_action,
				with: tool: "just,uv" },
			{ uses: #rust_cache },
			{ name: "Install kudune for running python tests"
				run: "just install-kudune" },
			{ name: "Install pyinfra for building Vaulta image",
				run: "uv tool install pyinfra" },
			{ name: "Run python tests",
				run: "just test-python" },
	    ],
	},
}

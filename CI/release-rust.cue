// SPDX-FileCopyrightText: 2026 DigiGaia SCCL
// SPDX-License-Identifier: AGPL-3.0-or-later

package CI

name: "release-rust"

// allow other workflows to call this one
on: workflow_call: {
	secrets: CARGO_REGISTRY_TOKEN: required: true
}

permissions:
    contents: "read"

_token: env: CARGO_REGISTRY_TOKEN: "${{ secrets.CARGO_REGISTRY_TOKEN }}"

jobs: {
	publish: {
		"runs-on": "ubuntu-latest",
		"timeout-minutes": 10,
		steps: [
			{ uses: #checkout },
			{ name: "Install Rust toolchain", uses: #rust_toolchain },
			{ name: "Cache dependencies", uses: #rust_cache },
			{ name: "Verify package", run: "cargo package" },
			{ if: #on_tag, name: "Publish crate `kudu-macros`", run: "cargo publish -p kudu-macros" } & _token,
			{ if: #on_tag, name: "Publish crate `kudu`", run: "cargo publish -p kudu" } & _token,
			{ if: #on_tag, name: "Publish crate `kudu-esr`", run: "cargo publish -p kudu-esr" } & _token,
			{ if: #on_tag, name: "Publish crate `kudune`", run: "cargo publish -p kudune" } & _token,
	    ]
	}
}

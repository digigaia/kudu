package CI

name: "build-cargo-crates"

on: {
    push: tags: ["*"],
    workflow_dispatch: null,
}

permissions:
    contents: "read"

_token: env: CARGO_REGISTRY_TOKEN: "${{ secrets.CARGO_REGISTRY_TOKEN }}"

jobs: {
	test: uses: "./.github/workflows/tests.yml",

	publish: {
		needs: "test",
		"runs-on": "ubuntu-latest",
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

package CI

// Shared definitions for all the CI jobs/workflows


// Runners

#macos_runner: {
	runner: "macos-latest",
	target: "aarch64",
}

#ubuntu_runner: {
	runner: "ubuntu-latest",
	target: "x86_64",
}

#ubuntu_arm_runner: {
	runner: "ubuntu-latest",
	target: "aarch64",
}

#ubuntu_2604_arm_runner: {
	runner: "ubuntu-26.04-arm",
	target: "aarch64",
}


// Dependencies

#checkout: "actions/checkout@v7"
#setup_python: "actions/setup-python@v7"
#upload_artifact: "actions/upload-artifact@v7"
#download_artifact: "actions/download-artifact@v8"
#attest_build_provenance: "actions/attest-build-provenance@v4"
#install_action: "taiki-e/install-action@7f4eb899022d8fe70b20c4f3de697aa85c309026"  // v2.85.11
#rust_cache: "Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6"  // v2.9.2
#maturin_action: "PyO3/maturin-action@e83996d129638aa358a18fbd1dfb82f0b0fb5d3b"  // v1.51.0
#setup_uv: "astral-sh/setup-uv@20cfd1bf945f4377ade1205e4dbc17946fc9a30d"  // v10.0.1

// define "runs-on" and a default timeout for all jobs
#job: {
	"runs-on": "${{ matrix.platform.runner }}"
	"timeout-minutes": int | *15,
	...
}

// useful define, writing it each time is a mouthful...
#on_tag: "${{ startsWith(github.ref, 'refs/tags/') }}"
#not_on_tag: "${{ !startsWith(github.ref, 'refs/tags/') }}"

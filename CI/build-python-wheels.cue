package CI

name: "build-python-wheels"

on: {
    push: tags: ["*"],
    workflow_dispatch: null,
}

permissions:
    contents: "read"


#build_steps: [
	{ uses: #checkout },
	{ uses: #setup_python, with: "python-version": 3.14	},
	{
		name: "Build wheels",
		uses: #maturin_action,
		with: {
			target: "${{ matrix.platform.target }}",
			"working-directory": "kudu-py",
			args: "--release --out dist --find-interpreter",
			sccache: #not_on_tag,
			...
		},
	},
    // {
	// 	name: "Build abi3t wheels for Python>=3.15",
	// 	uses: #maturin-action,
	// 	with: {
	// 		target: "${{ matrix.platform.target }}",
	// 		"working-directory": "kudu-py",
	// 		args: "--release --out dist -i python3.15t",
	// 		sccache: #not_on_tag,
	// 		...
	// 	},
	// },
	{
		name: "Upload wheels",
		uses: #upload_artifact,
		with: {
			name: string,
			path: "kudu-py/dist",
		},
	},
]


jobs: {
	test: uses: "./.github/workflows/tests.yml",

	linux: #job & {
		needs: "test"
		strategy: matrix: platform: [#ubuntu_runner, #ubuntu_arm_runner],
		steps: #build_steps & [_, _, {
			with: manylinux: "2_28"
		}, {
			with: name: "wheels-linux-${{ matrix.platform.target }}",
		}],
	},
	musllinux: #job & {
		needs: "test"
		strategy: matrix: platform: [#ubuntu_runner, #ubuntu_arm_runner],
		steps: #build_steps & [_, _, {
			with: manylinux: "musllinux_1_2"
		}, {
			with: name: "wheels-musllinux-${{ matrix.platform.target }}",
		}],
	},
	macos: #job & {
		needs: "test"
		strategy: matrix: platform: [#macos_runner],
		steps: #build_steps & [_, _, _, {
			with: name: "wheels-macos-${{ matrix.platform.target }}",
		}],
	},
	sdist: {
		needs: "test"
		"runs-on": "ubuntu-latest",
		"timeout-minutes": 3,
		steps: [
			{ uses: #checkout },
			{
				name: "Build sdist",
				uses: #maturin_action,
				with: {
					"working-directory": "kudu-py",
					command: "sdist",
					args: "--out dist",
				}
			}, {
				name: "Upload sdist",
				uses: #upload_artifact,
				with: name: "wheels-sdist",
				with: path: "kudu-py/dist",
			},
	    ],
	},
	release: {
		name: "Release"
		"runs-on": "ubuntu-latest",
		"timeout-minutes": 10
		if: "${{ startsWith(github.ref, 'refs/tags/') || github.event_name == 'workflow_dispatch' }}"
		needs: ["linux", "musllinux", "macos", "sdist"],
		permissions: {
			"id-token": "write",    // used to sign the release artifacts
			contents: "write",      // used to upload release artifacts
			attestations: "write",  // used to generate artifact attestation
		},
		steps: [
			{ uses: #download_artifact },
			{
				name: "Generate artifact attestation",
				uses: #attest_build_provenance,
				with: "subject-path": "wheels-*/*",
			}, {
				name: "Install uv",
				if: #on_tag,
				uses: #setup_uv,
			}, {
				name: "Publish to PyPI",
				if: #on_tag,
				run: "uv publish --trusted-publishing always 'wheels-*/*'"
			},
	    ],
	},
}

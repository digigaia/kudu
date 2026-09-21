package CI

import "list"

name: "tests"

on: {
    push: tags: ["*"],
    pull_request: branches: ["master"],
}

permissions:
    contents: "read"


// Steps
#prepare: [
	{ uses: #checkout },
	{ uses: #install_action,
	  with: tool: "just,uv,cargo-nextest" },
	{ uses: #rust_cache },
]


// define "runs-on" and a timeout for all jobs
jobs: [_]: #job


jobs: {
    tests: {
		"timeout-minutes": 3,
		strategy: matrix: platform: [
			#ubuntu_runner,
			#ubuntu_2604_arm_runner,
			#macos_runner,
	    ],
        steps: list.Concat([
    	    #prepare,
			[{ name: "Run tests", run: "just test" }],
		]),
	},

	"python-tests": {
		// do not run on `aarch64`, as the EOS VM doesn't run on it
		strategy: matrix: platform: [#ubuntu_runner],
        steps: list.Concat([
    	    #prepare,
			[
				{ name: "Install kudune for running python tests"
				  run: "just install-kudune" },
				{ name: "Install pyinfra for building Vaulta image",
				  run: "uv tool install pyinfra" },
				{ name: "Run python tests",
				  run: "just test-python" },
		    ]
		]),
	}

}

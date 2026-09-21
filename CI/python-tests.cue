package CI

import "list"

name: "tests"

on: {
    push: branches: ["master"],
    pull_request: branches: ["master"],
	workflow_call: null  // allow other workflows to call this one
}

permissions:
    contents: "read"


// Steps
#prepare: [
	{ uses: #checkout },
	{ uses: #install_action,
	  with: tool: "just,uv" },
	{ uses: #rust_cache },
]


// define "runs-on" and a timeout for all jobs
jobs: [_]: #job


jobs: {
	tests: uses: "./.github/workflows/tests.yml",

	"python-tests": {
		// do not run on `aarch64`, as the EOS VM doesn't run on it
		strategy: matrix: platform: [#ubuntu_runner],
		needs: "tests",
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

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
	  with: tool: "just,cargo-nextest" },
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
}

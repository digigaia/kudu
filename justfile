export RUST_BACKTRACE := "1"

doc_modules := "-p 'kudu*' -p syn@2 -p ureq -p serde -p serde_json -p snafu -p strum -p tracing"
open := if os() == "macos" { "open" } else { "xdg-open" }

# list recipes
default:
    just --list

# build the library and CLI tools in release mode
build-release:
    cargo build --features cli --release

# run tests using nextest
test:
    cargo nextest run

# generate documentation
doc:
    cargo doc --color always --no-deps {{doc_modules}}

# generate documentation and open it
doc-open: doc
    {{open}} target/doc/kudu/index.html

@_set_version file version:
    echo "Setting version to: {{version}} in {{file}}"
    sed 's/^version = ".*"$/version = "{{version}}"/' {{file}} | sponge {{file}}

@_set_version_in_deps file version:
    echo "Setting version to: {{version}} in {{file}} dependencies"
    sed 's/^\(kudu.*\)version = "[^"]*"\(.*\)$/\1version = "{{version}}"\2/' {{file}} | sponge {{file}}

# set the version number in all Cargo.toml files
set-version version: && \
    (_set_version "Cargo.toml" version) \
    (_set_version_in_deps "kudu/Cargo.toml" version) \
    (_set_version_in_deps "kudu-esr/Cargo.toml" version) \
    (_set_version_in_deps "kudune/Cargo.toml" version)
    @echo "Setting version to: {{version}}:"

# publish the project crates on crates.io
publish:
    #!/usr/bin/env bash
    set -euo pipefail
    # investigate `cargo-release` instead, or `cargo-smart-release` or `release-plz`
    echo "Make sure that you properly set the version number and do not forget to tag the release in git"
    read -p "Are you sure you want to proceed? (y/N) " confirm
    if [[ $confirm =~ ^[yY]$ ]]; then
        echo "Publishing crates..."
        cargo publish -p kudu-macros
        cargo publish -p kudu
        cargo publish -p kudu-esr
        cargo publish -p kudune
    fi


hyperfine_opts := "--shell=none --warmup 10"
abieos_path := "../abieos/build/tools"

abi := "kudu/src/abi/data/transaction_abi.json"
# bench_type := "bool"
# bench_hex := "01"
# bench_json := "true"
bench_type := "transaction"
bench_hex := "b2f4335b0bb10e87af9c000000000100a6823403ea3055000000572d3ccdcd01608c31c6187315d600000000a8ed323221608c31c6187315d6708c31c6187315d6010000000000000004535953000000000000"
bench_json := '{"expiration": "2018-06-27T20:33:54.000", "ref_block_num": 45323, "ref_block_prefix": 2628749070, "max_net_usage_words": 0, "max_cpu_usage_ms": 0, "delay_sec": 0, "context_free_actions": [], "actions": [{"account": "eosio.token", "name": "transfer", "authorization": [{"actor": "useraaaaaaaa", "permission": "active"}], "data": "608C31C6187315D6708C31C6187315D60100000000000000045359530000000000"}], "transaction_extensions": []}'

# perform some benchmarks
benchmark: build-release
    @echo "\n----==== Benchmarking hex -> JSON ====----\n"
    hyperfine {{hyperfine_opts}} \
        '{{abieos_path}}/generate_json_from_hex -f {{abi}} -x {{bench_type}} -h {{bench_hex}}' \
        'target/release/kuduconv from-hex --abi {{abi}} {{bench_type}} {{bench_hex}}'

    @echo "\n----==== Benchmarking JSON -> hex ====----\n"
    hyperfine {{hyperfine_opts}} \
        '{{abieos_path}}/generate_hex_from_json -f {{abi}} -x {{bench_type}} -j '"'"'{{bench_json}}'"'" \
        'target/release/kuduconv to-hex --abi {{abi}} {{bench_type}} '"'"'{{bench_json}}'"'"


api_endpoint := "https://vaulta.greymass.com"

@_download_abi name:
    echo "Downloading abi: {{name}}"
    curl --silent --json '{"account_name": "{{name}}"}' {{api_endpoint}}/v1/chain/get_abi | jq > "kudu/src/abi/data/{{name}}.json"

# download current ABIs from an API endpoint and store them in `src/abi/data`
download-abis: \
    (_download_abi "eosio") \
    (_download_abi "eosio.token") \
    (_download_abi "core.vaulta")

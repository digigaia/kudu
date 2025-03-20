export RUST_BACKTRACE := "1"

# list recipes
default:
    just --list

# run tests using nextest
test:
    cargo nextest run

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
    # DO NOT FORGET TO SET VERSION NUMBER AND GIT TAG
    echo "TODO!"

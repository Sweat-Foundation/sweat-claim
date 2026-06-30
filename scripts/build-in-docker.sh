#!/bin/bash
set -eox pipefail

echo ">> Building reproducible contract in Docker"

# cargo-near drives the Docker build itself, using the image and
# `container_build_command` from the `[package.metadata.near.reproducible_build]`
# section of contract/Cargo.toml. All changes must be committed to git.
cargo near build reproducible-wasm \
  --manifest-path contract/Cargo.toml \
  --out-dir res

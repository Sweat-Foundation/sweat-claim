#!/bin/bash
set -eox pipefail

echo ">> Building contract for integration tests"

# Same as build.sh, but with the `integration-test` feature enabled.
cargo near build non-reproducible-wasm \
  --no-abi \
  --locked \
  --features integration-test \
  --manifest-path contract/Cargo.toml \
  --out-dir res

#!/bin/bash
set -eox pipefail

echo ">> Building contract for integration tests"

# NOTE: integration tests are temporarily disabled while they are migrated off
# `nitka`/`sweat-model`, and the dedicated `integration-test` feature was removed.
# For now this builds the regular contract so the Makefile target keeps working.
cargo near build non-reproducible-wasm \
  --no-abi \
  --locked \
  --manifest-path contract/Cargo.toml \
  --out-dir res

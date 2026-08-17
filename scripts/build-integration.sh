#!/bin/bash
set -eox pipefail

echo ">> Building contract for integration tests"

# Builds the contract wasm consumed by integration-tests/ (its own cargo
# workspace, see Cargo.toml) — near-workspaces deploys this file via the
# CLAIM_WASM env var / default path in integration-tests/tests/common/prepare.rs.
cargo near build non-reproducible-wasm \
  --no-abi \
  --locked \
  --manifest-path contract/Cargo.toml \
  --out-dir res

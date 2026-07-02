#!/bin/bash
set -eox pipefail

echo ">> Building contract"

# `cargo near build` builds with the `--release` profile (see [profile.release] in
# the workspace Cargo.toml), generates the ABI, and writes the optimized
# `sweat_claim.wasm` (with the ABI embedded) into `res/`.
#
# near-sdk is pinned to 5.26.1 (matching sweat-token) rather than the newer 5.28.x
# line: from 5.27 onward, near-sdk's `abi` feature unconditionally pulls in
# near-global-contracts, whose StateInit type fails to compile under ABI
# generation with a recursive-type error in its BorshSchema derive — confirmed
# against both the published crate and near-sdk-rs's current main branch, with
# no upstream fix yet. 5.26.1 predates that dependency entirely.
#
# Our own view types (ClaimResultView, ClaimAvailabilityView, etc., in
# model/src/lib.rs) use `#[near(serializers = [json])]` instead of manual
# `Serialize`/`Deserialize` derives, since that macro also generates a
# properly crate-path-qualified, wasm32-target-gated `JsonSchema` impl
# whenever near-sdk-macros is built with its `abi` feature — which is exactly
# what ABI generation needs, without requiring a direct `schemars` dependency
# or manual target-gating in this repo.
cargo near build non-reproducible-wasm \
  --locked \
  --manifest-path contract/Cargo.toml \
  --out-dir res

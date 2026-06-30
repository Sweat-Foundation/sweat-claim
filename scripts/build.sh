#!/bin/bash
set -eox pipefail

echo ">> Building contract"

# `cargo near build` builds with the `--release` profile (see [profile.release] in
# the workspace Cargo.toml) and writes the optimized `sweat_claim.wasm` into `res/`.
# `--no-abi` is required: the forked near-sdk 4.x predates cargo-near's ABI support.
cargo near build non-reproducible-wasm \
  --no-abi \
  --locked \
  --manifest-path contract/Cargo.toml \
  --out-dir res

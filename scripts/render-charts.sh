#!/bin/bash
set -eox pipefail

echo ">> Render charts"

cargo test --package sweat_claim --lib claim::tests::demo::demo_evaporating -- --ignored
cargo test --package sweat_claim --lib claim::tests::demo::demo_continuous_evaporating -- --ignored
cargo test --package sweat_claim --lib claim::tests::demo::demo_evaporating_with_claim -- --ignored

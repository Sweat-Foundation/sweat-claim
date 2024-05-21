#!/bin/bash
set -eox pipefail

contract_name="sweat_claim"

commit="./res/${contract_name}_commit.wasm"
docker="./res/${contract_name}.wasm"

cp $docker $commit

make build-in-docker

commit_hash=$(openssl dgst -sha256 "$commit" | awk '{print $2}')
docker_hash=$(openssl dgst -sha256 "$docker" | awk '{print $2}')

if [ "$commit_hash" = "$docker_hash" ]; then
  echo "Binary hashes match."
else
  echo "The contract in commit hash does not match with hash of contract build in docker. You must call \`make dock\` command before submitting a PR." >&2
  exit 1
fi

help: ##@Miscellaneous Show this help
	@echo "Usage: make [target] ...\n"
	@perl -e '$(HELP_FUN)' $(MAKEFILE_LIST)

install: ##@Miscellaneous Install dependencies
	@npm i near-cli
	@cargo build

build: ##@Build Build the contract locally.
	./scripts/build.sh

build-in-docker: ##@Build Build reproducible artifact in Docker.
	./scripts/build-in-docker.sh

build-integration: ##@Build Build contract for integration tests.
	./scripts/build-integration.sh

dock: build-in-docker ##@Build Shorthand for `build-in-docker`

deploy: ##@Deploy Deploy the contract to dev account on Testnet.
	./scripts/deploy.sh

cov: ##@Testing Run unit tests with coverage.
	cargo llvm-cov --hide-instantiations --open --ignore-filename-regex tests.rs

test: ##@Testing Run unit tests.
	cargo test --package sweat_claim

integration: build-integration ##@Testing Run integration tests.
	cd integration-tests && cargo test

int: integration ##@Testing Shorthand for `integration`

integration-log: build-integration ##@Testing Run integration tests with logs (serial). Override level via RUST_LOG.
	cd integration-tests && RUST_LOG=$${RUST_LOG:-info,near_workspaces=warn} cargo test -- --nocapture --test-threads=1

fmt: ##@Chores Format the code using rustfmt nightly.
	cargo +nightly fmt --all

lint: ##@Chores Run lint checks with Clippy.
	./scripts/lint.sh

charts: ##@Chores Render charts.
	./scripts/render-charts.sh

hash: ##@Chores Check if contract in commit has valid hash.
	./scripts/check-contract-hash.sh

HELP_FUN = \
    %help; while(<>){push@{$$help{$$2//'options'}},[$$1,$$3] \
    if/^([\w-_]+)\s*:.*\#\#(?:@(\w+))?\s(.*)$$/}; \
    print"$$_:\n", map"  $$_->[0]".(" "x(20-length($$_->[0])))."$$_->[1]\n",\
    @{$$help{$$_}},"\n" for keys %help; \

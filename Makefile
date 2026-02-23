.PHONY: clean check fmt clippy test verify matrix-check tree docker-binaries drift cloudflare-targets-check ecosystem-check ecosystem-build ecosystem-test ecosystem-verify

CARGO_TARGET_DIR ?= $(CURDIR)/out/target
export CARGO_TARGET_DIR
EDGERUN := cargo run -p edgerun-cli -- --root .

clean:
	$(EDGERUN) ci --job clean-artifacts

check:
	cargo check --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

verify:
	$(EDGERUN) ci --job verify

drift:
	./scripts/check-workflow-drift.sh

cloudflare-targets-check:
	./scripts/verify-cloudflare-targets.sh

ecosystem-check:
	./scripts/ecosystem-workflow.sh check

ecosystem-build:
	./scripts/ecosystem-workflow.sh build

ecosystem-test:
	./scripts/ecosystem-workflow.sh test

ecosystem-verify:
	./scripts/ecosystem-workflow.sh verify

matrix-check:
	$(EDGERUN) ci --job matrix-check

tree:
	@find . -maxdepth 3 -type d | sort

docker-binaries:
	./scripts/build-docker-binaries.sh

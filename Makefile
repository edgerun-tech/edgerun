.PHONY: clean check fmt clippy test verify matrix-check tree docker-binaries

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

matrix-check:
	$(EDGERUN) ci --job matrix-check

tree:
	@find . -maxdepth 3 -type d | sort

docker-binaries:
	./scripts/build-docker-binaries.sh

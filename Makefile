.PHONY: clean check fmt clippy test verify matrix-check tree

CARGO_TARGET_DIR ?= $(CURDIR)/out/target
export CARGO_TARGET_DIR

clean:
	cargo run -p edgerun-cli -- --root . ci --job clean-artifacts

check:
	cargo check --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

verify:
	cargo run -p edgerun-cli -- --root . ci --job verify

matrix-check:
	cargo run -p edgerun-cli -- --root . ci --job matrix-check

tree:
	@find . -maxdepth 3 -type d | sort

.PHONY: check fmt clippy test tree

check:
	cargo check --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

tree:
	@find . -maxdepth 3 -type d | sort

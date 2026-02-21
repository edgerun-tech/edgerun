.PHONY: check fmt clippy test verify matrix-check tree

check:
	cargo check --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

verify:
	./scripts/verify.sh

matrix-check:
	./scripts/check-matrix-validation.sh

tree:
	@find . -maxdepth 3 -type d | sort

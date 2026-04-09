.PHONY: check test build release-build docker-build

# Default: workspace check
check:
	cargo check --workspace

# Run all Rust tests
test:
	cargo test --workspace

# Build all workspace binaries
build:
	cargo build --workspace --release

# Cross-compile release binaries
release-build:
	mkdir -p dist
	cargo build --release -p edgerun-node --bin edgerund
	cp target/release/edgerund dist/edgerund-linux-amd64

# Build container image from Rust source
docker-build:
	docker build -t edgerun-reference-core:local .

.PHONY: check test build release-build docker-build

# Default: workspace check
check:
	cd rust && cargo check --workspace

# Run all Rust tests
test:
	cd rust && cargo test --workspace

# Build all workspace binaries
build:
	cd rust && cargo build --workspace --release

# Cross-compile release binaries
release-build:
	mkdir -p dist
	cd rust && cargo build --release -p edgerun-node --bin edgerund
	cp rust/target/release/edgerund dist/edgerund-linux-amd64

# Build container image from Rust source
docker-build:
	docker build -t edgerun-reference-core:local .

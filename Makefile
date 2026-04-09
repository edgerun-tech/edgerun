.PHONY: check test build release-build docker-build
.PHONY: e2e e2e-full e2e-hardware e2e-mesh e2e-conformance e2e-runner

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

# ===========================================================================
# End-to-End Testing Targets
# ===========================================================================

# Run comprehensive E2E test suite (software only, no hardware required)
e2e:
	cargo test -p edgerun-e2e-full -- --ignored

# Run full E2E suite including hardware tests (requires HARDWARE_E2E=1)
e2e-full:
	HARDWARE_E2E=1 bash scripts/e2e-test-runner.sh --all

# Run hardware capability E2E tests (requires actual devices)
e2e-hardware:
	HARDWARE_E2E=1 cargo test -p edgerun-e2e-capability -- --ignored

# Run mesh integration tests (requires root for network namespaces)
e2e-mesh:
	sudo bash scripts/mesh-integration-test.sh

# Run protocol conformance corpus tests
e2e-conformance:
	cargo test -- conformance --ignored

# Run the interactive E2E test runner with menu
e2e-runner:
	bash scripts/e2e-test-runner.sh

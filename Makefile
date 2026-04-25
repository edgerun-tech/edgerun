.PHONY: check test build release docker-build install-ert version
.PHONY: e2e e2e-full e2e-hardware e2e-mesh e2e-conformance e2e-runner

# Run local CI checks (format, clippy, check, release build)
check:
	./scripts/ci-local.sh check

# Version bump (analyzes API changes and bumps version)
version:
	./scripts/ci-local.sh version

# Build release binary
release:
	./scripts/ci-local.sh release

# Run all Rust tests
test:
	cargo test --workspace

# Build all workspace binaries
build:
	cargo build --workspace --release

# Build and install ert binary to system PATH
install-ert:
	cargo build -p edgerun-oci-runtime --release
	sudo cp target/x86_64-unknown-linux-musl/release/ert /usr/local/bin/ert
	sudo chmod 755 /usr/local/bin/ert
	@echo "Installed ert to /usr/local/bin/ert"

# Build container image from Rust source
docker-build:
	docker build -t edgerun-reference-core:local .

# ===========================================================================
# End-to-End Testing Targets
# ===========================================================================

# Run comprehensive E2E test suite (software only, no hardware required)
e2e:
	cargo test -p edgerun-e2e -- --ignored

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

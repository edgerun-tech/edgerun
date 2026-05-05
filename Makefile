.PHONY: check test build release docker-build install-ert version
.PHONY: marketplace-localnet marketplace-localnet-status marketplace-localnet-stop marketplace-stress
.PHONY: e2e e2e-full e2e-mesh e2e-conformance e2e-runner

# Run local CI checks (format, clippy, check, release build)
check:
	./scripts/ci-local.sh check

# Version bump (analyzes API changes and bumps version)
version:
	./scripts/ci-local.sh version

# Build release binary
release:
	./scripts/ci-local.sh release

# Run all Rust tests and print an aggregate pass/fail summary
test:
	./scripts/test-summary.py

# Start a local Solana validator with marketplace programs preloaded
marketplace-localnet:
	./scripts/start-solana-validator.sh

# Show local marketplace validator status
marketplace-localnet-status:
	./scripts/start-solana-validator.sh status

# Stop the local marketplace validator
marketplace-localnet-stop:
	./scripts/start-solana-validator.sh stop

# Run local marketplace lifecycle stress; override with MARKETPLACE_STRESS_ITERATIONS=N
marketplace-stress: marketplace-localnet
	./scripts/stress-marketplace-local.sh

# Build all workspace binaries
build:
	cargo build --workspace --release

# Build and install ert binary to system PATH
install-ert:
	cargo build -p edgerun-oci --release
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

# Run mesh integration tests (requires root for network namespaces)
e2e-mesh:
	sudo bash scripts/mesh-integration-test.sh

# Run protocol conformance corpus tests
e2e-conformance:
	cargo test -- conformance --ignored

# Run the interactive E2E test runner with menu
e2e-runner:
	bash scripts/e2e-test-runner.sh

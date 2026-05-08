.PHONY: check test test-rust test-frontend test-all build release docker-build install-ert version
.PHONY: marketplace-localnet marketplace-localnet-status marketplace-localnet-stop marketplace-stress

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

# Run Rust workspace tests directly
test-rust:
	cargo test --workspace

# Run frontend tests
test-frontend:
	cd frontend && bun run test:run

# Run the local test suites that are part of the current workspace
test-all: test-rust test-frontend

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

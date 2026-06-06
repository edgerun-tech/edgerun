.PHONY: check test test-rust test-frontend test-all coverage coverage-frontend bench bench-list integration-test integration-test-list build release docker-build install-ert version
.PHONY: marketplace-localnet marketplace-localnet-status marketplace-localnet-stop marketplace-stress

# Run local CI checks (Rust format/lint/build plus frontend lock/lint/typecheck/test/build)
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

# Generate Rust and frontend coverage reports
coverage:
	./scripts/coverage-report.sh

# Generate frontend coverage only
coverage-frontend:
	./scripts/coverage-report.sh --frontend-only

# Run benchmark entry points and save a report under target/benchmarks
bench:
	./scripts/benchmark-report.py

# List benchmark entry points without running them
bench-list:
	./scripts/benchmark-report.py --list

# Run current Cargo integration test targets
integration-test:
	./scripts/integration-tests.py

# List current Cargo integration test targets
integration-test-list:
	./scripts/integration-tests.py --list

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

.PHONY: build-wat
W2W := wasm-tools parse
WAT_DIR := standards/build/wasm
RUNTIME_DIR := standards/system/runtime
build-wat:
	@echo "=== Building WAT modules ==="
	$(W2W) $(RUNTIME_DIR)/edgerun-core.wat -o $(RUNTIME_DIR)/edgerun-core.wasm
	$(W2W) $(WAT_DIR)/io/pipe-core.wat -o $(WAT_DIR)/io/pipe-core.wasm
	$(W2W) $(WAT_DIR)/io/frame-core.wat -o $(WAT_DIR)/io/frame-core.wasm
	$(W2W) $(WAT_DIR)/io/mux-core.wat -o $(WAT_DIR)/io/mux-core.wasm
	$(W2W) $(WAT_DIR)/io/pipeline-core.wat -o $(WAT_DIR)/io/pipeline-core.wasm
	$(W2W) $(WAT_DIR)/net/socket-core.wat -o $(WAT_DIR)/net/socket-core.wasm
	$(W2W) $(WAT_DIR)/encoding/encoding-core.wat -o $(WAT_DIR)/encoding/encoding-core.wasm
	$(W2W) $(WAT_DIR)/encoding/encoding-base64.wat -o $(WAT_DIR)/encoding/encoding-base64.wasm
	$(W2W) $(WAT_DIR)/encoding/encoding-base64url.wat -o $(WAT_DIR)/encoding/encoding-base64url.wasm
	$(W2W) $(WAT_DIR)/encoding/encoding-text.wat -o $(WAT_DIR)/encoding/encoding-text.wasm
	$(W2W) $(WAT_DIR)/crypto/crypto-sha1.wat -o $(WAT_DIR)/crypto/crypto-sha1.wasm
	$(W2W) $(WAT_DIR)/protocol/ws/ws-frame.wat -o $(WAT_DIR)/protocol/ws/ws-frame.wasm
	$(W2W) $(WAT_DIR)/protocol/ws/ws-stage.wat -o $(WAT_DIR)/protocol/ws/ws-stage.wasm
	$(W2W) $(WAT_DIR)/protocol/ws/ws-accept.wat -o $(WAT_DIR)/protocol/ws/ws-accept.wasm
	$(W2W) $(WAT_DIR)/protocol/ws/ws-client.wat -o $(WAT_DIR)/protocol/ws/ws-client.wasm
	$(W2W) $(WAT_DIR)/serialization/deflate-inflate.wat -o $(WAT_DIR)/serialization/deflate-inflate.wasm
	$(W2W) $(WAT_DIR)/pipeline/stage-registry.wat -o $(WAT_DIR)/pipeline/stage-registry.wasm
	$(W2W) $(WAT_DIR)/pipeline/wasm-interpreter.wat -o $(WAT_DIR)/pipeline/wasm-interpreter.wasm
	$(W2W) $(WAT_DIR)/pipeline/wasm-exec-stage.wat -o $(WAT_DIR)/pipeline/wasm-exec-stage.wasm
	$(W2W) $(WAT_DIR)/pipeline/process-wat-parse.wat -o $(WAT_DIR)/pipeline/process-wat-parse.wasm
	$(W2W) $(WAT_DIR)/text/wat-parse-core.wat -o $(WAT_DIR)/text/wat-parse-core.wasm
	@echo "=== All WAT modules built successfully ==="

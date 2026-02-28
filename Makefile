.PHONY: clean check fmt clippy test verify matrix-check tree docker-binaries drift workflow-refs-check required-checks-check cloudflare-targets-check ecosystem-check ecosystem-build ecosystem-test ecosystem-verify actions-local-list actions-local-run actions-local-dry-run actions-local-runtime-dry-run nodeos-initramfs nodeos-kernel-check nodeos-yubikey-cert nodeos-signed-uki nodeos-verify-uki nodeos-bootloader-efi nm-up nm-up-dev nm-status nm-logs nm-logs-nats nm-logs-mcp nm-down agent-launch agent-merge agent-test agent-context swarm-add-worker swarm-generate-executors-stack swarm-deploy-executors-stack code-update-event

CARGO_TARGET_DIR ?= $(CURDIR)/out/target
export CARGO_TARGET_DIR

clean:
	cargo clean

check:
	cargo check --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

verify:
	./scripts/ecosystem-workflow.sh verify

drift:
	./scripts/check-workflow-drift.sh

workflow-refs-check:
	./scripts/check-workflow-references.sh

required-checks-check:
	./scripts/check-required-checks.sh

cloudflare-targets-check:
	./scripts/verify-cloudflare-targets.sh

ecosystem-check:
	./scripts/ecosystem-workflow.sh check

ecosystem-build:
	./scripts/ecosystem-workflow.sh build

ecosystem-test:
	./scripts/ecosystem-workflow.sh test

ecosystem-verify:
	./scripts/ecosystem-workflow.sh verify

matrix-check:
	./scripts/ecosystem-workflow.sh check

tree:
	@find . -maxdepth 3 -type d | sort

docker-binaries:
	./scripts/build-docker-binaries.sh

actions-local-list:
	./scripts/actions-local-run.sh --list

actions-local-run:
	./scripts/actions-local-run.sh

actions-local-dry-run:
	./scripts/actions-local-run.sh --dry-run

actions-local-runtime-dry-run:
	./scripts/actions-local-run.sh --workflow ci.yml --event pull_request --job runtime-slo --dry-run

nodeos-initramfs:
	./scripts/nodeos/build-initramfs.sh

nodeos-kernel-check:
	./scripts/nodeos/verify-kernel-config.sh /usr/lib/modules/$$(uname -r)/build/.config

nodeos-yubikey-cert:
	./scripts/nodeos/create-yubikey-secureboot-cert.sh

nodeos-signed-uki:
	./scripts/nodeos/build-signed-uki.sh

nodeos-verify-uki:
	./scripts/nodeos/verify-signed-uki.sh

nodeos-bootloader-efi:
	./scripts/nodeos/build-edgerun-bootloader-efi.sh

nm-up:
	./scripts/node-manager-compose.sh up

nm-up-dev:
	./scripts/node-manager-compose.sh up-dev

nm-status:
	./scripts/node-manager-compose.sh status

nm-logs:
	./scripts/node-manager-compose.sh logs

nm-logs-nats:
	./scripts/node-manager-compose.sh logs-nats

nm-logs-mcp:
	./scripts/node-manager-compose.sh logs-mcp

nm-down:
	./scripts/node-manager-compose.sh down

agent-launch:
	@test -n "$(AGENT_ID)" || (echo "AGENT_ID is required"; exit 1)
	@test -n "$(PROMPT)" || (echo "PROMPT is required"; exit 1)
	./scripts/agents/launch-agent.sh "$(AGENT_ID)" "$(PROMPT)"

agent-merge:
	@test -n "$(AGENT_BRANCH)" || (echo "AGENT_BRANCH is required"; exit 1)
	./scripts/agents/merge-agent.sh "$(AGENT_BRANCH)" "$(TARGET_BRANCH)"

agent-test:
	@test -n "$(WORKSPACE_DIR)" || (echo "WORKSPACE_DIR is required"; exit 1)
	@test -n "$(PROFILE)" || (echo "PROFILE is required"; exit 1)
	./scripts/agents/test-executor.sh "$(WORKSPACE_DIR)" "$(PROFILE)"

agent-context:
	@test -n "$(MODE)" || (echo "MODE is required: pack|symbols|refs"; exit 1)
	@test -n "$(ARG)" || (echo "ARG is required"; exit 1)
	./scripts/agents/mcp-context.sh "$(MODE)" "$(ARG)"

swarm-add-worker:
	./scripts/swarm/add-worker-node.sh 10.13.37.2

swarm-generate-executors-stack:
	./scripts/swarm/generate-crate-executors-stack.sh

swarm-deploy-executors-stack:
	./scripts/swarm/deploy-crate-executors-stack.sh

code-update-event:
	./scripts/executors/publish-code-update.sh

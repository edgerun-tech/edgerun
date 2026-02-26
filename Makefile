.PHONY: clean check fmt clippy test verify matrix-check tree drift cloudflare-targets-check ecosystem-check ecosystem-build ecosystem-test ecosystem-verify actions-local-list actions-local-run actions-local-dry-run actions-local-runtime-dry-run nodeos-initramfs nodeos-kernel-check nodeos-yubikey-cert nodeos-signed-uki nodeos-verify-uki

CARGO_TARGET_DIR ?= $(CURDIR)/out/target
export CARGO_TARGET_DIR
EDGERUN := cargo run -p edgerun-cli -- --root .

clean:
	$(EDGERUN) ci --job clean-artifacts

check:
	cargo check --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

verify:
	$(EDGERUN) ci --job verify

drift:
	./scripts/check-workflow-drift.sh

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
	$(EDGERUN) ci --job matrix-check

tree:
	@find . -maxdepth 3 -type d | sort

actions-local-list:
	./scripts/actions-local-run.sh --list

actions-local-run:
	./scripts/actions-local-run.sh

actions-local-dry-run:
	./scripts/actions-local-run.sh --dry-run

actions-local-runtime-dry-run:
	./scripts/actions-local-check.sh

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

.PHONY: build install uninstall start-work integrate-work native-build

# Install into user-local bin; keep builds in a user-writable cache.
PREFIX := $(HOME)/.local
BINDIR := $(PREFIX)/bin
CARGO_TARGET_DIR ?= $(HOME)/.cache/edgerun-term/target
CARGO_BUILD_TARGET ?= $(shell rustc -vV | sed -n 's/^host: //p')
CARGO_TARGET_FLAGS := --target $(CARGO_BUILD_TARGET)
TERM_PACKAGE ?= edgerun-term-compositor
TERM_BIN ?= edgerun-term-compositor
SUDO ?=
CARGO ?= cargo

start-work:
	@echo "==> git status"
	@git status -sb || true
	@echo "==> build (debug)"
	$(CARGO) build -p $(TERM_PACKAGE) $(CARGO_TARGET_FLAGS) --target-dir $(CARGO_TARGET_DIR)

integrate-work:
	@echo "==> format"
	$(CARGO) fmt --all
	@echo "==> clippy"
	$(CARGO) clippy -p $(TERM_PACKAGE) $(CARGO_TARGET_FLAGS) --target-dir $(CARGO_TARGET_DIR)
	@echo "==> tests"
	$(CARGO) test -p $(TERM_PACKAGE) $(CARGO_TARGET_FLAGS) --target-dir $(CARGO_TARGET_DIR)
	@echo "==> build (release)"
	$(CARGO) build -p $(TERM_PACKAGE) $(CARGO_TARGET_FLAGS) --release --target-dir $(CARGO_TARGET_DIR)

build:
	$(CARGO) build -p $(TERM_PACKAGE) $(CARGO_TARGET_FLAGS) --release --target-dir $(CARGO_TARGET_DIR)

install: build
	$(SUDO) install -Dm755 $(CARGO_TARGET_DIR)/$(CARGO_BUILD_TARGET)/release/$(TERM_BIN) $(BINDIR)/edgerun-term

native-build:
	$(CARGO) build -p edgerun-term-native $(CARGO_TARGET_FLAGS) --release --target-dir $(CARGO_TARGET_DIR)

uninstall:
	$(SUDO) rm -f $(BINDIR)/edgerun-term

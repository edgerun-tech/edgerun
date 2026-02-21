APP_NAME := term
PREFIX ?= /usr
BINDIR := $(PREFIX)/bin

.PHONY: build install uninstall

build:
	cargo build --release

install: build
	install -Dm755 target/release/$(APP_NAME) $(BINDIR)/$(APP_NAME)

uninstall:
	rm -f $(BINDIR)/$(APP_NAME)

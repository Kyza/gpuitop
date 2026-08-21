PREFIX ?= /usr/local
DESTDIR ?=

CARGO_TARGET_DIR ?= $(CURDIR)/target
export CARGO_TARGET_DIR

SUDO = $(if $(DESTDIR),,$(if $(filter $(HOME)%,$(PREFIX)),,sudo))

BIN = gpuitop
BINDIR = $(DESTDIR)$(PREFIX)/bin
APPS = $(DESTDIR)$(PREFIX)/share/applications

.PHONY: build install uninstall

build:
	cargo build --release --features packaging

install: build
	$(SUDO) install -Dm755 $(CARGO_TARGET_DIR)/release/$(BIN) $(BINDIR)/$(BIN)
	$(SUDO) install -Dm644 $(CARGO_TARGET_DIR)/release/$(BIN).desktop $(APPS)/$(BIN).desktop

uninstall:
	$(SUDO) rm -f $(BINDIR)/$(BIN)
	$(SUDO) rm -f $(APPS)/$(BIN).desktop

# Makefile for codex-rs (cargo workspace root).
#
# One-shot usage:
#   make install    build a minimal release binary and drop it at ~/.local/bin/codex
#   make uninstall  move ~/.local/bin/codex to ~/.local/tmp/2del/
#   make clean      cargo clean
#
# The installed binary is built with all debuginfo stripped and symbols
# removed (≈ 270 MB). To get a debuggable fat binary (≈ 1.2 GB, matches
# upstream profile) use `make install-dev`.

PREFIX  ?= $(HOME)/.local
DESTDIR ?=
INSTALL_DIR := $(DESTDIR)$(PREFIX)/bin
INSTALL_BIN := $(INSTALL_DIR)/codex
BACKUP_DIR   := $(PREFIX)/tmp/2del
TIMESTAMP    := $(shell date +%Y%m%d-%H%M%S)

CARGO ?= cargo

# Hard-coded slim profile overrides (cargo 1.57+ env-var form).
# Drops debuginfo completely, keeps LTO=thin for size + speed.
SLIM_ENV := \
    CARGO_PROFILE_RELEASE_DEBUG=0 \
    CARGO_PROFILE_RELEASE_STRIP=symbols \
    CARGO_PROFILE_RELEASE_LTO=thin \
    CARGO_PROFILE_RELEASE_SPLIT_DEBUGINFO=off

.PHONY: all install install-dev uninstall reinstall clean
.DEFAULT_GOAL := install

all: install

install:
	@echo ">> building slim release (no debuginfo, symbols stripped)"
	@cd "$(CURDIR)" && env $(SLIM_ENV) $(CARGO) build --release -p codex-cli
	@echo ">> installing target/release/codex -> $(INSTALL_BIN)"
	@mkdir -p "$(INSTALL_DIR)"
	@if [ -e "$(INSTALL_BIN)" ]; then \
		mkdir -p "$(BACKUP_DIR)"; \
		backup="$(BACKUP_DIR)/codex.$(TIMESTAMP).bak"; \
		echo ">> backing up existing $(INSTALL_BIN) -> $$backup"; \
		mv -f "$(INSTALL_BIN)" "$$backup"; \
	fi
	@install -m 0755 target/release/codex "$(INSTALL_BIN)"
	@echo ">> installed: $(INSTALL_BIN)"
	@ls -lh "$(INSTALL_BIN)"

install-dev:
	@echo ">> building upstream-default release (≈ 1.2 GB, debuginfo=line-tables-only)"
	@cd "$(CURDIR)" && $(CARGO) build --release -p codex-cli
	@echo ">> installing target/release/codex -> $(INSTALL_BIN)"
	@mkdir -p "$(INSTALL_DIR)"
	@if [ -e "$(INSTALL_BIN)" ]; then \
		mkdir -p "$(BACKUP_DIR)"; \
		backup="$(BACKUP_DIR)/codex.$(TIMESTAMP).bak"; \
		echo ">> backing up existing $(INSTALL_BIN) -> $$backup"; \
		mv -f "$(INSTALL_BIN)" "$$backup"; \
	fi
	@install -m 0755 target/release/codex "$(INSTALL_BIN)"
	@echo ">> installed: $(INSTALL_BIN)"
	@ls -lh "$(INSTALL_BIN)"

uninstall:
	@if [ ! -e "$(INSTALL_BIN)" ]; then \
		echo ">> nothing to uninstall at $(INSTALL_BIN)"; \
		exit 0; \
	fi
	@mkdir -p "$(BACKUP_DIR)"
	@dest="$(BACKUP_DIR)/codex.$(TIMESTAMP).uninstalled"; \
	echo ">> moving $(INSTALL_BIN) -> $$dest"; \
	mv -f "$(INSTALL_BIN)" "$$dest"

reinstall: uninstall install

clean:
	@echo ">> cargo clean"
	@cd "$(CURDIR)" && $(CARGO) clean

.PHONY: help
help:
	@echo "Targets:"
	@echo "  make install     Build slim release + install to \$$HOME/.local/bin/codex (default)"
	@echo "  make install-dev Build upstream-default release + install (≈ 1.2 GB, for debugging)"
	@echo "  make reinstall   uninstall + install"
	@echo "  make uninstall   Move current binary to \$$HOME/.local/tmp/2del/"
	@echo "  make clean       cargo clean"
	@echo "  make help        Show this help"

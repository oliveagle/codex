# Codex Makefile
# Provides build and install targets with automatic version detection

.PHONY: build install check-version set-version clean help

# Get latest semver tag (rust-v0.X.Y format)
VERSION := $(shell git tag -l 'rust-v*' 2>/dev/null | grep -E '^rust-v0\.[0-9]+\.[0-9]+' | sort -V | tail -1 | sed 's/^rust-v//')
# Fallback if no semver tag found
ifeq ($(VERSION),)
VERSION := 0.0.0
endif
COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
COMMIT_COUNT := $(shell git rev-list --count HEAD 2>/dev/null || echo "0")
IS_DIRTY := $(shell git diff --quiet 2>/dev/null && echo "" || echo "-dirty")
FULL_VERSION := $(VERSION)-$(COMMIT_COUNT)-g$(COMMIT)$(IS_DIRTY)

# Current version in Cargo.toml
CARGO_VERSION := $(shell grep '^version = ' codex-rs/Cargo.toml 2>/dev/null | head -1 | sed 's/version = "\(.*\)"/\1/' || echo "0.0.0")

# Default target
all: build

# Show help
help:
	@echo "Codex Build System"
	@echo ""
	@echo "Targets:"
	@echo "  build        - Build codex with version from git"
	@echo "  install      - Build and install to /usr/local/bin"
	@echo "  check-version - Display version information"
	@echo "  set-version  - Update Cargo.toml with git version"
	@echo "  clean        - Clean build artifacts"
	@echo ""
	@echo "Current version: $(FULL_VERSION)"
	@echo "Cargo version:   $(CARGO_VERSION)"

# Check and display version info
check-version:
	@echo "=== Codex Version Info ==="
	@echo "Git version:    $(FULL_VERSION)"
	@echo "Base tag:       $(VERSION)"
	@echo "Commit:         $(COMMIT)"
	@echo "Commit count:   $(COMMIT_COUNT)"
	@echo "Dirty:          $(if $(IS_DIRTY),yes,no)"
	@echo "Cargo version:  $(CARGO_VERSION)"
	@echo "=========================="
	@if [ "$(CARGO_VERSION)" = "0.0.0" ]; then \
		echo ""; \
		echo "WARNING: Cargo.toml version is '0.0.0'"; \
		echo "Run 'make set-version' to set proper version"; \
	fi

# Set version in Cargo.toml
set-version:
	@echo "Setting version to: $(FULL_VERSION)"
	@sed -i 's/^version = "0.0.0"/version = "$(FULL_VERSION)"/' codex-rs/Cargo.toml
	@echo "Updated codex-rs/Cargo.toml"

# Build with version check
build: check-version
	@if [ "$(CARGO_VERSION)" = "0.0.0" ]; then \
		echo "Setting version before build..."; \
		$(MAKE) set-version; \
	fi
	@echo "Building codex $(FULL_VERSION)..."
	@cd codex-rs && cargo build --release
	@echo ""
	@echo "Build complete!"
	@echo "Binary: codex-rs/target/release/codex"
	@echo "Version: $$(./codex-rs/target/release/codex --version 2>/dev/null || echo 'unknown')"

# Install to /usr/local/bin
install: build
	@echo "Installing codex to /usr/local/bin..."
	@sudo cp codex-rs/target/release/codex /usr/local/bin/codex
	@echo "Installed: $$(codex --version)"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cd codex-rs && cargo clean
	@echo "Clean complete"

# Quick build without version check (for development)
dev:
	@echo "Building codex (dev mode)..."
	@cd codex-rs && cargo build
	@echo "Build complete: codex-rs/target/debug/codex"

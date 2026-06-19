#!/bin/bash
# Build and install codex with version from git
set -e

cd "$(dirname "$0")/.."

# Get version from git
VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^rust-v//' || echo "0.0.0")
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
COMMIT_COUNT=$(git rev-list --count HEAD 2>/dev/null || echo "0")
BUILD_DATE=$(date -u +"%Y%m%d")

# Build a semver-like version: <last-tag>-<commit-count>-<short-sha>
FULL_VERSION="${VERSION}-${COMMIT_COUNT}-g${COMMIT}"

echo "=== Build Info ==="
echo "Version:      ${FULL_VERSION}"
echo "Base tag:     ${VERSION}"
echo "Commit:       ${COMMIT}"
echo "Commit count: ${COMMIT_COUNT}"
echo "Build date:   ${BUILD_DATE}"
echo "=================="

# Backup original Cargo.toml
cp codex-rs/Cargo.toml codex-rs/Cargo.toml.bak

# Update workspace version in Cargo.toml
sed -i "s/^version = \"0.0.0\"/version = \"${FULL_VERSION}\"/" codex-rs/Cargo.toml

# Ensure we restore on exit
cleanup() {
    mv codex-rs/Cargo.toml.bak codex-rs/Cargo.toml
}
trap cleanup EXIT

# Build
cd codex-rs
cargo build --release

# Install
sudo cp target/release/codex /usr/local/bin/codex

echo ""
echo "Installed: $(codex --version)"

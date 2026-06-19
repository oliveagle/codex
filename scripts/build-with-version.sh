#!/bin/bash
# Build codex with version from git
set -e

cd "$(dirname "$0")/.."

# Get version from git
VERSION=$(git describe --tags --always --dirty 2>/dev/null || echo "unknown")
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

echo "Building codex with version: $VERSION"
echo "Commit: $COMMIT"
echo "Build date: $BUILD_DATE"

# Set version via environment variable
export CODEX_VERSION="$VERSION"
export CODEX_COMMIT="$COMMIT"
export CODEX_BUILD_DATE="$BUILD_DATE"

# Build
cd codex-rs
cargo build --release

echo ""
echo "Build complete!"
echo "Binary: codex-rs/target/release/codex"
echo "Version: $(./target/release/codex --version 2>/dev/null || echo 'unknown')"

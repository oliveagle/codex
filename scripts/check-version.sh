#!/bin/bash
# Check and display version info for codex builds
set -e

cd "$(dirname "$0")/.."

# Get version from git tags
VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^rust-v//' || echo "0.0.0")
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
COMMIT_COUNT=$(git rev-list --count HEAD 2>/dev/null || echo "0")
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
IS_DIRTY=$(git diff --quiet 2>/dev/null && echo "no" || echo "yes")
BUILD_DATE=$(date -u +"%Y%m%d")

# Build a semver-like version
FULL_VERSION="${VERSION}-${COMMIT_COUNT}-g${COMMIT}"
if [ "$IS_DIRTY" = "yes" ]; then
    FULL_VERSION="${FULL_VERSION}-dirty"
fi

# Current version in Cargo.toml
CARGO_VERSION=$(grep '^version = ' codex-rs/Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

echo "=== Codex Version Info ==="
echo "Git version:    ${FULL_VERSION}"
echo "Base tag:       ${VERSION}"
echo "Branch:         ${BRANCH}"
echo "Commit:         ${COMMIT}"
echo "Commit count:   ${COMMIT_COUNT}"
echo "Dirty:          ${IS_DIRTY}"
echo "Build date:     ${BUILD_DATE}"
echo "Cargo version:  ${CARGO_VERSION}"
echo "=========================="

# Check if Cargo.toml version needs update
if [ "$CARGO_VERSION" = "0.0.0" ]; then
    echo ""
    echo "WARNING: Cargo.toml version is '0.0.0'"
    echo "To set proper version, run: scripts/set-version.sh"
    exit 0
fi

if [ "$CARGO_VERSION" != "$FULL_VERSION" ]; then
    echo ""
    echo "WARNING: Cargo.toml version mismatch"
    echo "  Current:  ${CARGO_VERSION}"
    echo "  Expected: ${FULL_VERSION}"
    echo "Run: scripts/set-version.sh to update"
fi

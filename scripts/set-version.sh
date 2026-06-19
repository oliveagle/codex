#!/bin/bash
# Set version in Cargo.toml based on git tags
set -e

cd "$(dirname "$0")/.."

# Get version from git tags
VERSION=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^rust-v//' || echo "0.0.0")
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
COMMIT_COUNT=$(git rev-list --count HEAD 2>/dev/null || echo "0")
IS_DIRTY=$(git diff --quiet 2>/dev/null && echo "" || echo "-dirty")

# Build a semver-like version
FULL_VERSION="${VERSION}-${COMMIT_COUNT}-g${COMMIT}${IS_DIRTY}"

echo "Setting version to: ${FULL_VERSION}"

# Update workspace version in Cargo.toml
sed -i "s/^version = \"0.0.0\"/version = \"${FULL_VERSION}\"/" codex-rs/Cargo.toml

echo "Updated codex-rs/Cargo.toml"

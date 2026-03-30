#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="/home/fsos/Developer/Win-CodexBar"
RUST_MANIFEST="$REPO_ROOT/rust/Cargo.toml"

cargo fetch --manifest-path "$RUST_MANIFEST" >/dev/null

rustup target add x86_64-unknown-linux-gnu >/dev/null 2>&1 || true
rustup target add x86_64-pc-windows-gnu >/dev/null 2>&1 || true

mkdir -p /tmp/codexbar-mission

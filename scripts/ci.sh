#!/usr/bin/env bash
# Local CI: run before committing (Linux / macOS).
# Mirrors the old rust-tests.yml workflow, without touching GitHub Actions.
set -euo pipefail
cd "$(dirname "$0")"

echo "==> cargo fmt --check"
cargo fmt --all --check

echo "==> cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

echo "==> cargo test"
cargo test

echo "==> CI OK"

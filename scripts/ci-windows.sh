#!/usr/bin/env bash
# Local CI for the Windows target, run from Linux (or macOS).
# Verifies the Windows code compiles and passes clippy without needing a
# Windows machine. Tests cannot run here: they need a Windows runtime.
#
# First run: rustup target add x86_64-pc-windows-gnu
# (cargo check / clippy do not link, so mingw is not required)
set -euo pipefail
cd "$(dirname "$0")"

TARGET="x86_64-pc-windows-gnu"

if ! rustup target list --installed | grep -q "^${TARGET}$"; then
  echo "==> Target ${TARGET} not installed."
  echo "    Run: rustup target add ${TARGET}"
  echo "    Then re-run this script."
  exit 1
fi

echo "==> cargo check --target ${TARGET}"
cargo check --target "${TARGET}"

echo "==> cargo clippy --target ${TARGET} -- -D warnings"
cargo clippy --target "${TARGET}" -- -D warnings

echo "==> Windows CI OK (check + clippy only; tests need a Windows host)"

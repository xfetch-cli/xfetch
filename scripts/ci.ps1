# Local CI: run before committing (Windows PowerShell).
# Mirrors the old rust-tests.yml workflow, without touching GitHub Actions.
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "==> cargo fmt --check"
cargo fmt --all --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> cargo test"
cargo test
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> CI OK"

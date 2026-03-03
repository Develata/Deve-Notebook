Param()

$ErrorActionPreference = "Stop"

Write-Host "[lint-low-mem] clippy deve_web..."
cargo clippy -p deve_web --bin deve_web -- -D warnings

Write-Host "[lint-low-mem] clippy deve_cli..."
cargo clippy -p deve_cli --bin deve_cli -- -D warnings

Write-Host "[lint-low-mem] fmt check..."
cargo fmt --all -- --check

Write-Host "[lint-low-mem] done"

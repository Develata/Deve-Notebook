@echo off
setlocal

echo [lint-low-mem] clippy deve_web...
cargo clippy -p deve_web --bin deve_web -- -D warnings || exit /b 1

echo [lint-low-mem] clippy deve_cli...
cargo clippy -p deve_cli --bin deve_cli -- -D warnings || exit /b 1

echo [lint-low-mem] fmt check...
cargo fmt --all -- --check || exit /b 1

echo [lint-low-mem] done
endlocal

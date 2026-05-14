# Desktop Process Adapter Fake Runtime Harness

Date: 2026-05-14

## Scope

Added a Desktop-only fake process runtime harness for `native-packaging` tests.

No real child process runtime was opened.

## Code Changes

- Added `apps/desktop/src/process_runtime.rs` as a test-only `native-packaging` fake runtime harness.
- Added `apps/desktop/src/process_runtime_test.rs` covering the process runtime state machine.
- Bound the new tests into `check-native-packaging-gate.sh`.

## Guarded Boundaries

- Default Desktop build still has no process runtime module.
- Fake runtime does not import `std::process`, `tokio::process`, `Command::new`, or call any process spawn API.
- Runtime state alone does not unlock writable UI; Desktop shell still requires the existing writer/readiness gates.
- Session handoff failure remains fatal.
- Health probe failure and process exit consume retry budget.

## Verification

- `cargo test --locked -p deve_desktop --features native-packaging process_runtime -- --nocapture`
- `scripts/check-native-process-adapter-gate.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/plan-coverage.sh --summary-missing-plan-ref`
- `cargo fmt --check`
- `git diff --check`

## Next Step

Continue platform package verification for Desktop AppImage/macOS/Windows hosts, or run a broader native regression refresh before entering real process runtime design.

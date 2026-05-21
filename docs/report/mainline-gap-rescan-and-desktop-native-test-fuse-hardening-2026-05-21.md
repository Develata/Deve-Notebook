# Mainline Gap Rescan And Desktop Native Test Fuse Hardening - 2026-05-21

## Scope

- Baseline: `b56ab358 Record local full regression refresh`.
- Route: local non-Apple mainline gap rescan after Windows / Android / WSL evidence and full regression closure.
- Explicitly kept closed: Apple/macOS/iOS evidence, signing, store release, physical-device readiness, native authority writes, Mobile process runtime, Android process runtime, Web Git writer, and server-backed Settings API.
- `docs/plan/` was not modified.

## Selection

The rescan found no new blocking guard failure and no new unblocked Current Web/server `MUST` gap. The most useful local code item was the audit finding that Desktop native test files were close to the 500-line hard fuse:

- `apps/desktop/src/process_runtime_test.rs`: 492 lines before split.
- `apps/desktop/src/service_bootstrap_test.rs`: 464 lines before split.

This was selected as a low-risk hardening item because the Desktop native route is still active, and the change can be made without expanding runtime authority or platform readiness claims.

## Changes

- Split `apps/desktop/src/process_runtime_test.rs` into:
  - `apps/desktop/src/process_runtime_test/mod.rs`
  - `apps/desktop/src/process_runtime_test/support.rs`
  - `apps/desktop/src/process_runtime_test/lifecycle.rs`
  - `apps/desktop/src/process_runtime_test/validation.rs`
  - `apps/desktop/src/process_runtime_test/shell_gate.rs`
- Split `apps/desktop/src/service_bootstrap_test.rs` into:
  - `apps/desktop/src/service_bootstrap_test/mod.rs`
  - `apps/desktop/src/service_bootstrap_test/support.rs`
  - `apps/desktop/src/service_bootstrap_test/orchestration.rs`
  - `apps/desktop/src/service_bootstrap_test/loopback_http.rs`

The split is test-layout only. Assertions, shell authority boundaries, native session secrecy checks, process retry semantics, and writer-gate behavior remain unchanged.

## Verification

```bash
cargo fmt --check
cargo test --locked -p deve_desktop --features native-packaging process_runtime_test -- --nocapture
cargo test --locked -p deve_desktop --features native-packaging service_bootstrap_test -- --nocapture
cargo test --locked -p deve_desktop --features native-packaging -- --nocapture
./scripts/plan-coverage.sh
./scripts/check-native-process-adapter-gate.sh
```

Result:

- `process_runtime_test`: passed, `15` tests.
- `service_bootstrap_test`: passed, `10` tests.
- Full `deve_desktop` native-packaging tests: passed, `69` tests.
- `plan-coverage.sh`: passed with `0` blocking violations; size soft warnings dropped from `27` in the preceding full regression to `25`.
- `check-native-process-adapter-gate.sh`: passed.

Execution note:

- A first `plan-coverage.sh` run was attempted before staging the file split. Because the script scans Git index paths, it still saw the old deleted files and failed on missing paths. After staging the split, the same gate passed.
- A first targeted Desktop test was run concurrently with `cargo fmt --check` and hit a transient Windows Cargo `rlib` lookup error. The same test passed when rerun serially after formatting.

## Conclusion

The local mainline gap rescan did not identify a new feature gap that should override the closed boundaries. The selected Desktop native test fuse hardening is complete and verified. The next local follow-up can continue with smaller Desktop native source-file soft warnings, while Apple evidence remains blocked on a real Apple-capable host.

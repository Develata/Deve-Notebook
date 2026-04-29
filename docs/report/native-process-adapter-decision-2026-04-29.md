# Native Process Adapter Decision - 2026-04-29

## Result

P3-10 native process adapter decision is closed for the current default build:
real child-process runtime is deferred until the native packaging gate opens.

The current desktop/mobile skeletons remain no-Tauri/no-runtime contract crates.
They consume `NativeServiceSupervisor`, but they do not spawn, own, signal,
restart, or persist a backend child process.

## Implemented Guard

- `deve_core::native_adapter::NativeProcessAdapterDecision` records the current
  decision as `DeferredUntilPackagingGate`.
- `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY` fixes:
  - `child_process_runtime_enabled = false`
  - `packaging_gate_required = true`
  - `authority_writes_allowed = false`
- Desktop and mobile unit tests assert that the default build keeps the real
  process adapter deferred and authority-free.

## Rationale

- The supervisor contract already fixes readiness, health probe, retry budget,
  and session handoff semantics without introducing process lifecycle coupling.
- A real child-process adapter needs packaging/runtime decisions for stdout and
  stderr capture, crash handling, signals, port reuse, config/profile/vault
  selection, and platform lifecycle behavior.
- Mobile adds OS-specific background and process lifetime constraints that
  cannot be represented correctly in the no-runtime skeleton.
- Process running must never imply application writable. Writable state remains
  gated by auth status, `/api/node/role`, repo handshake, writer-ready, and
  current `scope_nonce`.

## Still Not Implemented

- No `std::process::Command` based child-process launcher.
- No platform signal handling or child process tree cleanup.
- No stdout/stderr log bridge.
- No crash-loop supervisor over a real backend process.
- No Tauri/Tauri Mobile runtime dependency.
- No native authority over ledger, vault, source-control, search, `.git`, or
  `.notegit`.

## Verification

- `cargo test -p deve_core native_adapter`
- `cargo test -p deve_desktop`
- `cargo test -p deve_mobile`
- `cargo check --workspace --all-targets --all-features`
- `cargo fmt --all --check`
- `scripts/plan-coverage.sh`
- `git diff --check`

## Next

The next native-track decision is whether to open the actual packaging
dependency gate. If it opens, the first batch must remain feature-gated in
`apps/desktop` or `apps/mobile` and must not grant core authority writes.

# Desktop Packaging Dependency Spike - 2026-05-14

本报告记录 `NPG-1 Desktop Packaging Dependency Spike`。`docs/plan/` 仍是唯一权威；本批只打开 Desktop dependency spike，不打开 Mobile dependency spike，不打开 native process adapter。

## Scope

- Plan basis: `14_tech_stack.md#native-packaging-dependency-gate`, `08_ui_design_02_desktop.md#desktop-packaging-dependency-gate-decision`, `08_ui_design_03_mobile.md#mobile-packaging-dependency-gate-decision`.
- Code scope: `apps/desktop`, `apps/mobile`, `crates/core/src/native_adapter`, native guard scripts.
- Non-goal: spawn backend process, supervise child process, write ledger/vault/source-control/search/Git authority, claim macOS/Windows/Android release readiness.

## Result

- `apps/desktop` may depend on `tauri` and `tauri-build` only through `native-packaging`.
- Default desktop build remains no-Tauri.
- Mobile remains dependency-deferred and no-Tauri.
- `CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY` distinguishes Desktop and Mobile dependency gates.
- `CURRENT_NATIVE_PROCESS_ADAPTER_POLICY` remains `DeferredUntilPackagingGate`.

## Verification

- `cargo check -p deve_desktop --features native-packaging`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `cargo test -p deve_core native_adapter -- --nocapture`
- `cargo test -p deve_desktop --features native-packaging packaging -- --nocapture`
- `cargo test -p deve_mobile --features native-packaging packaging -- --nocapture`

## Residual Gates

- `NPG-2` must validate Desktop shell-level packaging acceptance before Mobile dependency spike.
- Mobile dependency spike remains blocked until Desktop acceptance closes.
- Native process adapter remains a separate gate; packaging dependency success does not imply service supervision readiness.

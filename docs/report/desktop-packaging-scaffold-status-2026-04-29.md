# Desktop Packaging Scaffold Status - 2026-04-29

## Result

P3-10 desktop packaging scaffold plan split has landed.

Implemented scope:

- `apps/desktop` now exposes a feature-gated `packaging` scaffold behind
  `native-packaging`.
- The scaffold records the first planned desktop dependency batch:
  `tauri` + `tauri-build`.
- The scaffold records packaging-only capabilities: window shell, menu bar,
  system tray, installer, and auto-update.
- The scaffold records forbidden authorities: ledger, vault, source-control,
  search index, Git mirror, and `.notegit`.
- No actual Tauri dependency or runtime import was added.
- `scripts/check-native-track-boundary.sh` now verifies that the scaffold exists
  while still blocking real packaging dependency/import leakage.

Boundary:

- This is not a runnable Tauri app.
- `native-packaging` remains a future gate, not a default feature.
- No-packaging shell tests remain the correctness boundary for
  endpoint/session/bootstrap/readiness behavior.
- Packaging acceptance is limited to shell/platform concerns and cannot grant
  business authority.

## Verification

Commands run:

```bash
scripts/check-native-track-boundary.sh
cargo test -p deve_desktop --all-features
cargo test -p deve_desktop
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-targets --all-features
cargo fmt --all --check
scripts/plan-coverage.sh
git diff --check
```

Observed result:

- Native boundary script passed.
- Desktop no-feature and all-features tests passed.
- Workspace all-targets all-features check passed.
- Workspace all-targets all-features tests passed.
- Plan coverage passed with zero blocking violations.

## Next Work

The next P3-10 step should mirror this split for mobile packaging:

1. Add a `native-packaging` mobile packaging scaffold.
2. Keep the mobile no-packaging lifecycle/readiness tests authoritative.
3. Keep mobile permission bridge, push, file picker, and store distribution as
   packaging acceptance only.

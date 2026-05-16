# Full Regression Gate After Mainline Queue Closure

Date: 2026-05-16

## Scope

This gate ran after the mainline queue was cleared by:

- JS editor widget i18n bridge.
- Global shortcut parity.
- Indirect sync source attribution envelope.
- Dockerfile build strategy drift closure.

`docs/plan/` was not modified in this batch.

## Fixes During Gate

The first full-feature clippy pass exposed narrow structure issues. They were fixed before final verification:

- Sync hello outbound follow-up arguments were consolidated into `OutboundSyncContext`.
- `SyncPushSnapshot` handler arguments were consolidated into `SyncPushSnapshotInput`.
- Global shortcut event handling now receives `GlobalShortcutSignals` as one context object.
- Editor widget i18n bridge now builds an `EditorWidgetI18n` copy package, so Rust facade copy is consumed in non-wasm and wasm builds.

These changes are behavior-preserving cleanup for clippy-clean full-feature builds.

## Verification

Rust foundation:

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --all-targets --all-features -- -D warnings`

Targeted checks after fixes:

- `cargo test --locked -p deve_cli sync_transfer_snapshot -- --nocapture`
- `cargo test --locked -p deve_cli sync_hello -- --nocapture`
- `cargo test --locked -p deve_web global_shortcut -- --nocapture`
- `cargo test --locked -p deve_web editor_widget_copy -- --nocapture`

Runtime, release, native, mobile, and domain guards:

- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-release-audit-gate.sh`
- `bash scripts/smoke-web-release-build.sh`
- `bash scripts/smoke-runtime-release-info.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/plan-coverage.sh`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-ai-baseline.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-browser-prefs-boundary.sh`
- `bash scripts/check-cli-settings-baseline.sh`
- `bash scripts/check-dev-data-health-baseline.sh`
- `bash scripts/check-diff-color-baseline.sh`
- `bash scripts/check-graph-baseline.sh`
- `bash scripts/check-i18n-formatting-baseline.sh`
- `bash scripts/check-i18n-hardcoded-baseline.sh`
- `bash scripts/check-large-doc-baseline.sh`
- `bash scripts/check-rendering-baseline.sh`
- `bash scripts/check-search-baseline.sh`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-source-control-smoke-hygiene.sh`
- `bash scripts/check-storage-repo-baseline.sh`
- `bash scripts/check-ui-dashboard-refresh-baseline.sh`
- `bash scripts/check-ui-desktop-baseline.sh`
- `bash scripts/check-ui-disconnect-baseline.sh`
- `bash scripts/check-ui-focus-baseline.sh`
- `bash scripts/check-ui-spa-routing-baseline.sh`
- `bash scripts/check-ui-token-baseline.sh`
- `bash scripts/check-ui-z-index-baseline.sh`
- `bash scripts/check-ws-structured-errors.sh`

Docker:

- `bash scripts/smoke-docker-release.sh`

## Result

- All Rust foundation checks passed.
- All targeted checks passed.
- All runtime, release, native, mobile, and domain guards passed.
- `scripts/check-release-audit-gate.sh` passed in local diagnostic mode; `cargo-audit` is unavailable on this host.
- `scripts/smoke-runtime-release-info.sh` skipped because no local runtime server was reachable at `127.0.0.1:3001`.
- `scripts/smoke-docker-release.sh` skipped because the Docker daemon was not reachable on this host.
- `scripts/plan-coverage.sh` remains blocking-clean: 0 blocking violations, 18 soft size warnings.

## Decision

The mainline is regression-clean for the current local environment.

The next step should be a plan-led platform work selection batch. It should choose the first concrete Docker/Desktop/Android implementation scope from current plan and report evidence, while keeping native authority writes and real process runtime closed unless a post-gate runtime feature is explicitly opened.

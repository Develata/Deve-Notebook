# Mainline Feature Selection After Platform Evidence Refresh - 2026-05-17

本报告记录当前 platform evidence refresh 后的主线 feature implementation selection。`docs/plan/` 未修改。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/`, `docs/acceptance-cases/`, guard scripts, current code, latest GitHub platform evidence.
- Runtime head: `1a48cc9ad92097e8c25609f72f1850fa1742e08e`.
- Boundary: 不打开 native process runtime、native authority writes、signing/notarization、store release、physical-device readiness、Web Git writer 或 server-backed Settings API。

## Platform Evidence Input

Latest platform evidence remains shell-only.

- Docker Smoke `25980828115`: success at `818b5c94`.
- Native Target Host all `25980828117`: success at `818b5c94`.
- Native Target Host Android package evidence rerun `25981241624`: success at `546fd7e2`.
- Android diagnostics hardening rerun `25981541267`: success at `1a48cc9a`.

The Android diagnostics hardening only guarantees the package-build coverage marker remains visible even when emulator smoke fails later; it does not open native process runtime or native authority writes.

## Verification

Ran:

- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/plan-coverage.sh`
- `bash scripts/check-network-baseline.sh`
- `bash scripts/check-auth-baseline.sh`
- `bash scripts/check-source-control-baseline.sh`
- `bash scripts/check-settings-local-feedback-baseline.sh`
- `bash scripts/check-repo-file-ops-baseline.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-mobile-baseline.sh`
- `bash scripts/check-native-process-adapter-gate.sh`
- `bash scripts/check-native-packaging-gate.sh`
- `bash scripts/smoke-runtime-happy-path.sh`
- `bash scripts/smoke-runtime-recovery-path.sh`

Results:

- Acceptance bindings: automated `149`, feature walkthrough `54`, manual `0`, unbound soft `0`.
- Feature operation paths: passed.
- Architecture registry: `72` flows, active drift `0`.
- Plan coverage: blocking violations `0`, soft warnings `18`, dangling `plan_ref` `0`, i18n leaks `0`.
- Network, auth, source control, settings local feedback, repo file operations, release, mobile, native process adapter, native packaging, runtime happy path, and runtime recovery guards passed.

## Findings

- No unbound acceptance case was found.
- No active architecture drift was found.
- No blocking plan coverage violation was found.
- No small unblocked Current Web/server MUST gap was found.
- Existing Future / Optional surfaces remain correctly gated:
  - Web Git writer.
  - Server-backed Settings API.
  - Native child-process runtime.
  - Signing, notarization, store release, TestFlight, Play Store.
  - Physical-device readiness.
  - Native authority writes.

## Decision

Do not start an unclear feature batch only to create implementation work.

Next batch: **Full Regression Gate Refresh After Platform Evidence Diagnostics**.

Rationale:

- Current Web/server user-facing Current surface has no newly identified small implementation gap.
- Platform evidence is green and shell-only; opening post-gate platform runtime would be a separate explicit selection.
- The safest next step is to run the full regression gate at `1a48cc9a`, then rescan for the next concrete Current gap.

## Exit Criteria For Next Batch

- `cargo fmt --check`
- `cargo test --locked`
- `cargo clippy --all-targets --all-features -- -D warnings`
- acceptance, architecture, feature-path, plan coverage guards.
- domain baselines.
- release/native/mobile guards.
- Web release build.
- runtime happy/recovery smoke.
- diff hygiene.

# Mainline Gap Scan After Full Regression

Date: 2026-05-16

## Scope

- Source of truth: `docs/plan/`.
- Inputs: `docs/features/operations/`, `docs/acceptance-cases/`, guard scripts, current code, and three read-only subagent reviews.
- Non-goal: reopen `docs/plan/`, native process runtime, native authority writes, signing/notarization, app store, physical-device readiness, Graph renderer, Calculation Runtime, or MCP runtime.

## Verification Snapshot

- `scripts/plan-coverage.sh --write-report`: 0 blocking violations, 0 missing production `plan_ref`, 0 dangling `plan_ref`, 17 known soft size warnings.
- `bash scripts/check-architecture-registry.sh`: 72 flows, 0 active drift.
- `bash scripts/check-acceptance-bindings.sh`: 110 automated, 60 feature walkthrough, 29 manual, 0 unbound.
- `bash scripts/check-feature-operation-paths.sh`: ok.
- `bash scripts/smoke-runtime-happy-path.sh`: ok.
- `bash scripts/smoke-runtime-recovery-path.sh`: ok.
- `bash scripts/check-release-baseline.sh`: ok.
- `bash scripts/check-native-packaging-gate.sh`: ok.
- `bash scripts/check-native-process-adapter-gate.sh`: ok.
- `bash scripts/check-mobile-platform-package-preflight.sh`: ok.
- GitHub Actions `Docker Smoke` run `25963571993`: success.

## Findings

### P0

None.

### P1

1. JS editor widget visible text bypasses the `t::*` i18n facade.
   - Plan: `11_i18n.md#i18n-facade-contract`.
   - Evidence: `apps/web/js/extensions/code_toolbar.js` hardcodes `Copy Code` / `More Actions`; `code_menu.js` hardcodes `No actions available`; `mermaid.js` hardcodes `Mermaid Error`.
   - Impact: locale switching cannot cover CodeMirror widget surfaces, so I18N-001 / I18N-003 remain incomplete for visible editor UI.
   - Minimal batch: add a small JS i18n bridge, route widget copy through it, and extend `scripts/check-i18n-hardcoded-baseline.sh` to cover `apps/web/js/extensions`.

2. Global shortcut implementation is narrower than the plan table.
   - Plan: `13_settings.md#keyboard-shortcuts`, `11_i18n.md` related command `Global: Toggle Language`.
   - Evidence: `apps/web/src/shortcuts/global/handlers.rs` only handles `Ctrl/Cmd+Shift+P`, `Ctrl/Cmd+P`, and `Ctrl/Cmd+Shift+K`.
   - Impact: documented shortcuts for language toggle, outline toggle, and sidebar toggle are not available from the global handler.
   - Minimal batch: add `Ctrl/Cmd+L`, `Ctrl/Cmd+Shift+O`, and `Ctrl/Cmd+B` routing to existing intents, with focused tests and no new business semantics.

3. Indirect sync attribution is not payload-signature-backed.
   - Plan: `05_network.md` §10.2 and §10.5.
   - Evidence: current `SyncPush` authorization checks requested `source_peer_id` and header consistency, but `EncryptedOp` has no source signature / signer / payload hash envelope.
   - Impact: current NET-011 fail-closed boundary blocks unrequested relay sources, but a future relay path still needs proof that payload attribution belongs to the declared source rather than the transport peer.
   - Minimal batch: design and implement a source-signed envelope for `SyncPush` / `SyncPushSnapshot`, then add a forged relay payload negative test.

### P2

1. Dockerfile build strategy now intentionally differs from `15_release.md` cargo-chef wording.
   - Evidence: `Dockerfile` no longer installs or runs `cargo-chef`; `scripts/check-release-baseline.sh` now rejects cargo-chef.
   - Current result: direct locked Docker build passed GitHub Docker Smoke.
   - Required decision: either approve a small `docs/plan/15_release.md` correction, or restore a cargo-chef path that can pass locked CI.

2. Repo/branch switch ack frames are not self-contained scope-stamped payloads.
   - Evidence: `ServerMessage::RepoSwitched` / `BranchSwitched` rely on `switch_nonce` and follow-up projection messages, not a full `repo_id / branch / scope_nonce` ack.
   - Impact: current Web path is guarded by nonce and later scoped messages; protocol-level self-containment can be improved without blocking current smoke.

3. Remote shadow apply atomicity plan_ref points at maintenance code more than the apply path.
   - Evidence: actual single-transaction apply is in `crates/core/src/ledger/shadow_transfer.rs`, while reverse coverage prominently points to `shadow_maintenance_runtime.rs`.
   - Impact: behavior appears covered, but traceability is less precise than the plan-code bijection target.

4. Native signed/notarized release, updater artifacts, native process runtime, and mobile store/device readiness remain post-gate.
   - Evidence: current native workflows produce shell-only evidence; process runtime and native authority writes remain closed.
   - Impact: platform release completeness is not a Web/server mainline blocker, but remains the next large delivery track after current P1s.

## Next Batch Selection

Execute P1 in this order:

1. JS editor widget i18n bridge and hardcoded-copy guard.
2. Global shortcut parity for language, outline, and sidebar.
3. Indirect sync source-signed envelope design/implementation.

Rationale: the first two are small, current, user-visible Web gaps. The sync envelope is a real plan-level correctness item but larger and should be isolated after the small UI/i18n closure.

# Mainline Gap Rescan After Platform Evidence Diagnostics - 2026-05-17

本报告记录 platform evidence diagnostics 全量回归通过后的主线缺口复扫。`docs/plan/` 未修改。

## Scope

- Input gate: `docs/report/full-regression-gate-refresh-after-platform-evidence-diagnostics-2026-05-17.md`.
- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/`, `docs/acceptance-cases/`, guard scripts, current code, latest platform evidence.
- Boundary: 不打开 Web Git writer、server-backed Settings API、native process runtime、signing、physical-device 或 native authority writes。

## Verification

Ran:

- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`
- `rg` scan over `docs/plan`, `docs/features`, `docs/acceptance-cases`, and latest reports for Future / Planned / Optional / post-gate / manual / unbound markers.

Results:

- Acceptance bindings: automated `149`, feature walkthrough `54`, manual `0`, unbound soft `0`.
- Feature operation paths: passed.
- Architecture registry: `72` flows, active drift `0`.
- Full regression gate at `590994e6` passed before this rescan.

## Findings

- No new unblocked Current Web/server MUST gap was found.
- No acceptance, feature-path, or architecture blocker was found.
- Remaining high-value surfaces are explicitly gated:
  - `13_settings.md`: server-backed Settings API remains Planned / Optional.
  - `12_commands.md`: Web Git writer remains closed; Git mirror command entries stay CLI-only notice.
  - `08_ui_design_02_desktop.md` and `08_ui_design_03_mobile.md`: Desktop/Mobile process runtime, native authority writes, and packaging-runtime expansion are post-gate.
  - `15_release.md`: signing, notarization, store release, TestFlight, Play Store, and physical-device readiness remain outside the current baseline.
  - `10_ai_agent.md`: Trusted CLI remains optional/default-off and must not become a core dependency.

## Decision

Do not select another Web/server feature batch without a concrete Current gap.

Next batch: **Desktop / Android Post-Gate Scope Decision After Mainline Green**.

Rationale:

- Current Web/server baseline is green under full regression and has no small unblocked MUST gap.
- The user-facing next strategic question is whether to explicitly open Desktop/Android post-gate work.
- That requires a scope decision first, because opening native process runtime, native authority writes, signing, store, or physical-device gates changes the authority and release boundary.

## Required Scope Questions For Next Batch

- Desktop first slice: shell-only packaging hardening, process runtime spike, or local service lifecycle?
- Android first slice: emulator package/startup hardening, Tauri mobile package lane, or native capability preflight?
- Authority boundary: keep native shell as thin client, or explicitly introduce a native-owned service lifecycle?
- Release boundary: stay unsigned/internal, or start signing/notarization/store preflight as a separate release lane?
- Test environment: GitHub target-host only, local WSL smoke only, or both.

## Non-Goals

- No immediate native authority write path.
- No physical-device readiness claim.
- No signed release or store release claim.
- No server-backed Settings API.
- No Web Git writer.

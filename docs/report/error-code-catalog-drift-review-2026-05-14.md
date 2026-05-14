# Error Code Catalog Drift Review - 2026-05-14

本报告记录错误码目录的只读 drift review。`docs/plan/` 仍是唯一权威；本批次不修改 plan。

## Scope

- Source of truth: `docs/plan/11_i18n.md#i18n-error-code-catalog`.
- Code scope: `crates/core/src/protocol/error.rs`, `apps/web/src/i18n/server_error.rs`.
- Cross-check scope: `docs/plan/`, `docs/features/`, `docs/acceptance-cases/`, `scripts/check-*.sh`.
- Non-goal: edit `docs/plan/`, add a failing drift guard, or change error semantics.

## Result

Current counts:

- Plan catalog: 38 codes.
- `ServerErrorCode` enum: 40 codes.
- Web i18n mapping: 40 codes.

Code and Web i18n are internally consistent: every enum code has a Web localized message.

Plan catalog is no longer missing the previously reported core codes:

- `SC_STALE_SCOPE`
- `DOC_CONTEXT_INVALID`
- `SYNC_REPO_ROUTE_MISMATCH`
- `SYNC_SNAPSHOT_REQUIRED`
- `SYNC_INVALID_PAYLOAD`
- `PLUGIN_UNKNOWN_PLUGIN`
- `PLUGIN_CAPABILITY_DENIED`
- `PLUGIN_RUNTIME_ERROR`
- `PLUGIN_SERIALIZATION_ERROR`

## Remaining Drift

Two implemented and user-facing codes are missing from the plan catalog.

| Code | Current implementation | Expected plan location |
|:---|:---|:---|
| `SC_COMMIT_DIFF_UNPROJECTABLE` | `ServerErrorCode`, Web i18n mapping, Source Control proxy mapping | `11_i18n.md` Source Control Errors |
| `GRAPH_DEGRADED_PROJECTION_REQUIRED` | `ServerErrorCode`, Web i18n mapping, graph API mapping, `REL-008` acceptance | `11_i18n.md` new Graph Errors section |

## Decision

This batch does not edit `docs/plan/`.

Recommended next plan-authorized patch:

- Add `SC_COMMIT_DIFF_UNPROJECTABLE` to `11_i18n.md` Source Control Errors.
- Add a `Graph Errors (GRAPH_*)` section to `11_i18n.md`.
- Add `GRAPH_DEGRADED_PROJECTION_REQUIRED` to that section.
- After the plan patch, add or extend a guard that compares `ServerErrorCode` serde names against the `11_i18n.md` catalog.

## Verification

Ran:

- Parsed `docs/plan/11_i18n.md` catalog rows.
- Parsed `#[serde(rename = "...")]` names from `crates/core/src/protocol/error.rs`.
- Parsed `ServerErrorCode::*` coverage in `apps/web/src/i18n/server_error.rs`.
- Searched `docs/plan/`, `docs/features/`, `docs/acceptance-cases/`, and `scripts/check-*.sh` for error-code references.

Result: no code/Web i18n gap; plan catalog drift remains the two codes listed above.

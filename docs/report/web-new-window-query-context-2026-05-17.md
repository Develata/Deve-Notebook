# Web New Window Query Context - 2026-05-17

本报告记录 Web shell `Open in New Window` query context 的实现闭合。`docs/plan/` 未修改。

## Scope

- `docs/plan/08_ui_design_01_web.md` 要求新窗口链接保留现有 query context，并正确追加 `doc=...`。
- 本批只处理 Web shell URL state 与 DocList selection。
- 不新增后端 authority，不改 repo scope、ledger、source-control、native process、signing 或 physical-device gate。

## Issue

- Explorer menu 已生成 `?doc=...` URL。
- Web 初始 DocList selection 未消费 `doc` query path。
- 如果当前 URL 已有旧 `doc` 参数，旧实现会追加第二个 `doc`，导致新窗口 query state 可能保留 stale document selection。

## Changes

- `Open in New Window` URL builder now replaces an existing `doc` query param before appending the selected document path.
- DocList reconciliation now selects the document matching decoded `?doc=...` when no current document is open.
- Pending-created document selection remains higher priority than query-driven selection.
- Malformed or empty `doc` query values fail soft and do not select a document.

## Verification

Ran:

- `cargo fmt --check`
- `cargo test -p deve_web new_window_url -- --nocapture`
- `cargo test -p deve_web message_projection::doc::tests -- --nocapture`
- `cargo test -p deve_web pending_created_doc_selection_wins_over_query_doc -- --nocapture`
- `cargo clippy -p deve_web --all-targets -- -D warnings`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-architecture-registry.sh`
- `bash scripts/plan-coverage.sh`

Results:

- URL builder tests: passed.
- DocList query selection tests: passed.
- Pending-created priority test: passed.
- Web clippy: passed.
- Acceptance bindings: automated `146`, feature walkthrough `54`, manual `0`, unbound `0`.
- Feature operation paths: passed.
- Architecture registry: `72` flows, `0` active drift.
- Plan coverage: `0` blocking violations, `18` soft warnings.

## Decision

Web new-window query context is closed for the current contract slice.

Next batch: **Mainline Gap Rescan After Web New Window Query Context Closure**.

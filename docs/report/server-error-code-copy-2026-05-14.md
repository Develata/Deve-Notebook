# Server Error Code Copy Closure - 2026-05-14

本报告记录 `ServerError.detail` UI 展示依赖的修复。`docs/plan/` 仍是唯一权威；本文件只记录当前实现事实。

## Scope

- Plan basis: `11_i18n.md#i18n-error-code-catalog`.
- Code scope: Web runtime protocol handling, plugin response completion, Source Control notices, AI Chat operation docs.
- Non-goal: change backend error classification, remove debug `detail`, or edit `docs/plan/`.

## Finding

Plan 要求：后端 `detail` 可作为调试信息返回，但前端展示与分支判断必须仅依赖 `code`。

The scan found user-visible paths that still rendered backend `detail`:

- Search unavailable banner appended `ServerError.detail`.
- Generic protocol banner appended `ServerError.detail`.
- Chat plugin error placeholder appended `ServerError.detail`.
- Source Control server notices retained backend `detail` as display hint.

## Fixes

- Search unavailable banner now uses `t::server_error::message(locale, error.code)`.
- Generic protocol banner now shows only the code-mapped localized message; `detail` remains log-only.
- PluginResponse error completion now appends the code-mapped localized message to the assistant placeholder or partial stream.
- SourceControlNotice created from server errors now drops backend `detail`; local UI-only markers remain internal and are not backend natural-language detail.
- AI Chat feature and acceptance wording now assert code-copy visibility instead of backend detail visibility.

## Verification

Ran:

- `cargo test -p deve_web message_protocol -- --nocapture`
- `cargo test -p deve_web message_dispatch_runtime -- --nocapture`
- `cargo test -p deve_web plugin_text_response -- --nocapture`
- `cargo test -p deve_web source_control_notice -- --nocapture`
- `bash scripts/check-ai-baseline.sh`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `cargo fmt --check`
- `git diff --check`

Results: pass. Plan coverage remains `0` blocking violations with `17` existing soft size warnings.

## Residual Notes

- `ServerError.detail` still exists for logs, wire decode tests, and backend debug context.
- The known error-code catalog drift reported by the read-only review requires a separate plan-authorized documentation batch, because this batch intentionally did not edit `docs/plan/`.

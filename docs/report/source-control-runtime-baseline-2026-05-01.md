# Source Control Runtime Baseline - 2026-05-01

本报告合并 watcher debounce、external file pending、rename pair 与 conflict precondition guard 的短状态报告。

## Current Boundary

- 外部 markdown 新增、删除、重复 touch 不得产生重复语义 pending 或重复刷新广播。
- 外部 rename pair 必须折叠为稳定单条 rename row，并保留 `doc_id` / `renamed_from`。
- Pending upsert 的语义变化信号决定是否广播 `FsChangeDetected`。
- `ResolveConflict` 必须服务端校验 pending entry 存在且 `has_conflict=true`；非 conflict pending 返回 scoped structured error，不改 pending/staged。
- Imported conflict 的 `KeepFs` / `KeepLedger` 必须通过 Source Control 显式流完成，不得绕过 stage/commit。

## Verified Surfaces

- Core watcher duplicate semantic event tests。
- Isolated `serve --dev` + Chrome MCP external new-file smoke。
- Isolated `serve --dev` + Chrome MCP rename-pair smoke。
- Source Control conflict resolution runtime tests。
- Non-conflict `ResolveConflict` 请求必须以结构化 `SC_CONFLICT_TARGET_MISSING` 拒绝；拒绝不得 stage entry，也不得 discard workspace content。
- Conflict target resolution 使用精确 resolved pending entry，不信任 client-selected path。
- 证据测试包括 `resolve_conflict_rejects_non_conflict_pending_entry` 与 `source_control_git_import_conflict_test`。

## Retired Source Reports

- `source-control-conflict-precondition-guard-2026-05-01.md`
- `source-control-external-new-file-runtime-smoke-2026-04-30.md`
- `source-control-rename-pair-runtime-smoke-2026-04-30.md`
- `watcher-external-new-file-debounce-status-2026-04-30.md`
- `watcher-rename-pair-debounce-status-2026-04-30.md`

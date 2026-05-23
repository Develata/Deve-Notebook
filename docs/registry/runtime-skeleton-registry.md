<!-- Generated: 2026-05-20 -->

# Runtime Skeleton Registry

## Status

本文件是 runtime 名称、收敛状态、当前代码承载路径与 tracking task 的受控登记表。
它连接 `docs/plan/` 的 refactor target、`docs/tasks/18_infra_runtime.md` 的责任带
与当前代码，但不替代任何 plan 章节的规范性约束。

## Rules

- `runtime` 名称必须与 plan 章节中的 `Refactor Target` 或本表既有名称一致。
- `status` 只能使用下表枚举。
- `current_module_path` 必须指向当前真实承载位置；分散承载时列主要路径，不写
  尚未存在的目标路径。
- `tracking_task` 指向当前迁移蓝图；没有明确任务时写 `待分配`。
- 代码路径迁移、runtime 独立成模块、或状态跨档时，必须同步更新本表。

## Status Values

| Status | Meaning |
|---|---|
| `已收敛` | 已有独立命名模块或窄模块组，边界与 runtime 名称基本一致。 |
| `部分承载` | 行为已存在，但分散在多个模块，或模块名尚未按 runtime 收敛。 |
| `未启动` | 尚无稳定代码承载位置。 |
| `抽象分层` | 当前是跨模块架构层，不应假装已有单一 runtime 模块。 |

## Runtime Registry

| Runtime | Status | Current module path | Tracking task | Boundary |
|---|---|---|---|---|
| `authority_storage_runtime` | `已收敛` | `crates/core/src/ledger/manager/authority_storage_runtime.rs` | `docs/tasks/18_infra_runtime.md` | Ledger append validation 与 authority table 边界。 |
| `projection_persistence_runtime` | `已收敛` | `crates/core/src/sync/projection_persistence_runtime.rs` | `docs/tasks/18_infra_runtime.md` | 从 ledger fold 派生 projection、workspace writeback 与 drift explanation。 |
| `watcher_runtime` | `部分承载` | `crates/core/src/sync/watcher/`; `crates/core/src/watcher.rs` | `docs/tasks/18_infra_runtime.md` | 只把外部文件事件归一化为 `pending_fs_ops`。 |
| `repair_runtime` | `已收敛` | `crates/core/src/ledger/manager/repair_runtime.rs` | `docs/tasks/18_infra_runtime.md` | Degraded、quarantine 与 repair action 边界。 |
| `projection_repair_runtime` | `已收敛` | `crates/core/src/sync/projection_repair_runtime.rs` | `docs/tasks/18_infra_runtime.md` | Projection rebuild、projection repair 与 repair diagnostics。 |
| `repo_catalog_runtime` | `已收敛` | `crates/core/src/ledger/manager/repo_catalog_runtime.rs` | `docs/tasks/18_infra_runtime.md` | Repo identity、catalog listing 与 selector 输入。 |
| `repo_scope_runtime` | `已收敛` | `crates/core/src/ledger/manager/repo_scope_runtime.rs` | `docs/tasks/18_infra_runtime.md` | Branch、`scope_nonce` 与 writable state。 |
| `session_runtime` | `部分承载` | `apps/cli/src/server/auth/handlers/session.rs`; `apps/cli/src/server/ws/auth.rs` | `docs/tasks/18_infra_runtime.md` | User session 与 WebSocket session lifecycle。 |
| `auth_gateway` | `部分承载` | `apps/cli/src/server/auth/` | `docs/tasks/18_infra_runtime.md` | HTTP/WS 入口鉴权、cookie/JWT 与安全头。 |
| `browser_auth_runtime` | `部分承载` | `apps/web/src/app/auth_monitor.rs`; `apps/web/src/api/auth_probe.rs` | `docs/tasks/18_infra_runtime.md` | Browser auth probe、session refresh 与 unauthorized recovery。 |
| `transport_runtime` | `部分承载` | `apps/cli/src/server/handlers/sync/`; `apps/web/src/hooks/use_core/effects/message_runtime.rs`; `apps/web/src/hooks/use_core/effects/message_runtime_sync/` | `docs/tasks/18_infra_runtime.md` | WS/HTTP transport、message classification 与 protocol gate。 |
| `repo_scope_sync_runtime` | `部分承载` | `crates/core/src/sync/repo_scoped.rs`; `apps/cli/src/server/repo_scope/sync.rs`; `apps/cli/src/server/handlers/sync/hello/scope/` | `docs/tasks/18_infra_runtime.md` | Repo-scoped handshake、scope cleanup 与 stale discard。 |
| `browser_peer_runtime` | `部分承载` | `apps/web/src/hooks/use_core/effects/message_runtime_sync/`; `apps/web/src/hooks/use_core/effects/message_dispatch_runtime/` | `docs/tasks/19_repo_refactor_blueprint.md` | WebLightPeer state、browser message dispatch 与 repo-scoped protocol state。 |
| `relay_proxy_runtime` | `未启动` | `未启动` | `待分配` | Relay/proxy routing only；不得改写 payload source attribution。 |
| `browser_document_runtime` | `部分承载` | `apps/web/src/editor/sync/`; `apps/web/src/editor/hook_runtime.rs` | `docs/tasks/20_web_thin_client_ledger_migration.md` | Browser document snapshot、history、route payload 与 doc-scoped sync。 |
| `pending_overlay_runtime` | `部分承载` | `apps/web/src/hooks/use_core/pending/`; `apps/web/src/editor/sync/history_replay.rs` | `docs/tasks/20_web_thin_client_ledger_migration.md` | Browser session pending overlay；不得写入 `pending_fs_ops`。 |
| `write_confirmation_runtime` | `部分承载` | `apps/web/src/hooks/use_core/callbacks_sync/write.rs`; `apps/web/src/editor/sync/history_resend.rs`; `apps/cli/src/server/handlers/document/` | `docs/tasks/20_web_thin_client_ledger_migration.md` | Write readiness、ack、reject 与 committed-but-writeback-failed 分类。 |
| `source_control_runtime` | `已收敛` | `crates/core/src/ledger/manager/source_control_runtime.rs`; `crates/core/src/source_control/` | `docs/tasks/18_infra_runtime.md` | 消费 `pending_fs_ops` / `GitImportRequested` 并生成 Deve stage/commit intent。 |
| `diff_session_runtime` | `部分承载` | `crates/core/src/source_control/diff.rs`; `apps/cli/src/server/handlers/source_control/diff/`; `apps/web/src/components/diff_view/` | `docs/tasks/18_infra_runtime.md` | Diff session 只通过 `source_control_runtime` 进入 ledger commit。 |
| `merge_runtime` | `部分承载` | `crates/core/src/ledger/merge/`; `apps/cli/src/server/handlers/merge/` | `docs/tasks/18_infra_runtime.md` | Merge lifecycle 只通过 source-control authority path 收敛。 |
| `backup_locator_runtime` | `部分承载` | `crates/core/src/backup/`; `apps/cli/src/commands/backup.rs` | `待分配` | repo/branch URL 到 WebDAV/S3 backup locator 的解析与只读检查；不得成为 repo authority。 |
| `backup_pack_runtime` | `未启动` | `未启动` | `待分配` | 从 ledger/snapshot authority 规划加密 backup packs；不得读取 stale UI state。 |
| `backup_restore_runtime` | `未启动` | `未启动` | `待分配` | verify/decrypt 后生成 restore candidate；不得直接写 local branch 或 Projection Workspace。 |
| `document_runtime` | `部分承载` | `apps/cli/src/server/handlers/document/`; `apps/web/src/editor/` | `docs/tasks/20_web_thin_client_ledger_migration.md` | OpenDoc、snapshot、history 与 edit intent。 |
| `render_projection_runtime` | `部分承载` | `apps/web/src/editor/`; `apps/web/src/components/outline_render/` | `docs/tasks/18_infra_runtime.md` | Projection-only rendering state 与 render hints。 |
| `widget_bridge_runtime` | `部分承载` | `apps/web/js/extensions/`; `apps/web/src/editor/` | `docs/tasks/18_infra_runtime.md` | Widget bridge 与 renderer extension，不拥有 authority。 |
| `outline_projection_runtime` | `部分承载` | `apps/web/src/components/outline_render/`; `apps/web/src/hooks/use_outline.rs` | `docs/tasks/18_infra_runtime.md` | Outline projection only；不得成为 document authority。 |
| `ui_shell` | `抽象分层` | `apps/web/src/components/`; `apps/desktop/src/shell/`; `apps/mobile/src/shell/` | `docs/tasks/18_infra_runtime.md` | View intent、layout shell、panel/focus/stacking 管理。 |
| `application_control` | `抽象分层` | `apps/web/src/hooks/use_core/`; `apps/desktop/src/shell/`; `apps/mobile/src/shell/` | `docs/tasks/19_repo_refactor_blueprint.md` | Control 编排、typed intent routing 与 runtime 状态汇聚。 |
| `feature_runtime` | `抽象分层` | `apps/web/src/components/sidebar/source_control/`; `apps/web/src/components/command_palette/`; `apps/web/src/components/dashboard/` | `docs/tasks/19_repo_refactor_blueprint.md` | Feature-local state machines，不直接持有 authority。 |

## Notes

- `source_control_runtime -> Git mirror bridge` 仍以 `docs/plan/07_diff_logic.md#git-mirror-lifecycle` 为权威边界；本表只登记 runtime 当前承载。
- `browser_peer_runtime -> browser_document_runtime -> pending_overlay_runtime -> write_confirmation_runtime` 是 Web write confirmation 主链；当前代码仍是分散承载。
- `document_runtime -> render_projection_runtime -> widget_bridge_runtime / outline_projection_runtime` 必须保持 projection-only 边界。
- `ui_shell -> application_control -> feature_runtime` 当前是抽象分层，不应为了满足表格而创建空模块。

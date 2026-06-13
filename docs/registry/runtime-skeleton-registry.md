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
| `server_runtime_assembly` | `部分承载` | `apps/cli/src/server/runtime/`; `apps/cli/src/server/start.rs`; `apps/cli/src/server/setup.rs`; `apps/cli/src/server/router.rs`; `apps/cli/src/server/static_files.rs`; `apps/cli/src/server/metrics.rs`; `apps/cli/src/server/p2p/`; `apps/cli/src/server/prewarm.rs`; `apps/cli/src/server/tree_state.rs` | `docs/tasks/19_repo_refactor_blueprint.md` | Server composition root、auth/plugin/sync/watcher/tree/metrics/static-router 装配；handlers 不得承担 runtime assembly。 |
| `browser_auth_runtime` | `部分承载` | `apps/web/src/app/auth_monitor.rs`; `apps/web/src/api/auth_probe.rs` | `docs/tasks/18_infra_runtime.md` | Browser auth probe、session refresh 与 unauthorized recovery。 |
| `transport_runtime` | `部分承载` | `apps/cli/src/server/handlers/sync/`; `apps/web/src/hooks/use_core/effects/message_runtime.rs`; `apps/web/src/hooks/use_core/effects/message_runtime_sync/` | `docs/tasks/18_infra_runtime.md` | WS/HTTP transport、message classification 与 protocol gate。 |
| `repo_scope_sync_runtime` | `部分承载` | `crates/core/src/sync/repo_scoped.rs`; `apps/cli/src/server/repo_scope/sync.rs`; `apps/cli/src/server/handlers/sync/hello/scope/` | `docs/tasks/18_infra_runtime.md` | Repo-scoped handshake、scope cleanup 与 stale discard。 |
| `session_client` | `部分承载` | `apps/web/src/runtime/session_client/`; `apps/web/src/hooks/use_core/effects/`; `apps/web/src/api/` | `docs/tasks/19_repo_refactor_blueprint.md` | Web transport/session readiness、handshake gate 与 reconnect coordination；client adapter，不保存业务真相。 |
| `relay_proxy_runtime` | `部分承载` | `crates/core/src/protocol/relay_proxy.rs`; `apps/cli/src/server/handlers/sync/transfer.rs`; `apps/cli/src/server/handlers/sync/snapshot.rs` | `待分配` | Relay/proxy route admission only；不得改写 payload source attribution。 |
| `scope_client` | `部分承载` | `apps/web/src/runtime/scope_client/`; `apps/web/src/hooks/use_core/`; `apps/web/src/hooks/use_core/effects_switch/` | `docs/tasks/19_repo_refactor_blueprint.md` | Browser repo/branch/doc scope、`scope_nonce`、scope prefs 与 stale-scope recovery；只消费 server/core authority。 |
| `document_client` | `部分承载` | `apps/web/src/runtime/document_client/`; `apps/web/src/runtime/document/`; `apps/web/src/editor/sync/`; `apps/web/src/editor/hook_runtime.rs` | `docs/tasks/20_web_thin_client_ledger_migration.md` | Browser document snapshot、pending overlay、ack/reject 与 navigation guard；client-side coordination adapter。 |
| `pending_overlay_runtime` | `已收敛` | `apps/web/src/runtime/document/pending/` | `docs/tasks/20_web_thin_client_ledger_migration.md` | Browser session pending overlay；不得写入 `pending_fs_ops`。 |
| `write_confirmation_runtime` | `已收敛` | `apps/web/src/runtime/document/write_state.rs`; `apps/web/src/runtime/document/confirm.rs`; `apps/cli/src/server/handlers/document/write_confirmation.rs` | `docs/tasks/20_web_thin_client_ledger_migration.md` | Ack、reject 与 committed-but-writeback-failed 的生命周期分类合同；write readiness 由 handshake/scope 带前置门控，不在本带收口。 |
| `source_control_runtime` | `已收敛` | `crates/core/src/ledger/manager/source_control_runtime.rs`; `crates/core/src/source_control/` | `docs/tasks/18_infra_runtime.md` | 消费 `pending_fs_ops` / `GitImportRequested` 并生成 Deve stage/commit intent。 |
| `diff_session_runtime` | `部分承载` | `crates/core/src/source_control/diff.rs`; `apps/cli/src/server/handlers/source_control/diff/`; `apps/web/src/components/diff_view/` | `docs/tasks/18_infra_runtime.md` | Diff session 只通过 `source_control_runtime` 进入 ledger commit。 |
| `merge_runtime` | `部分承载` | `crates/core/src/ledger/merge/`; `apps/cli/src/server/handlers/merge/` | `docs/tasks/18_infra_runtime.md` | Merge lifecycle 只通过 source-control authority path 收敛。 |
| `backup_locator_runtime` | `部分承载` | `crates/core/src/backup/locator.rs`; `crates/core/src/backup/provider.rs`; `crates/core/src/backup/root.rs`; `crates/core/src/backup/layout.rs`; `crates/core/src/backup/discovery.rs`; `crates/core/src/backup/secret.rs`; `crates/core/src/backup/binding.rs`; `apps/cli/src/commands/backup.rs`; `apps/cli/src/commands/backup/bind.rs`; `apps/cli/src/commands/backup/unbind.rs` | `待分配` | repo/branch URL 到 WebDAV/S3 backup locator、provider adapter dispatch、backup root manifest、remote layout diagnostics、readonly branch discovery、credential/key ref、branch binding 的解析与 dry-run bind/unbind 检查；不得成为 repo authority。 |
| `backup_pack_runtime` | `部分承载` | `crates/core/src/backup/pack.rs`; `crates/core/src/backup/protection.rs`; `crates/core/src/backup/upload.rs`; `apps/cli/src/commands/backup/run.rs` | `待分配` | 从 ledger/snapshot authority 规划 backup pack manifest、artifact protection metadata 与 upload state admission；CLI 只暴露 dry-run 计划，不得读取 stale UI state、直接访问 provider 或上传。 |
| `backup_restore_runtime` | `部分承载` | `crates/core/src/backup/verification.rs`; `crates/core/src/backup/restore.rs`; `crates/core/src/backup/restore_flow.rs`; `apps/cli/src/commands/backup/restore.rs` | `待分配` | verify/decrypt 后生成 verification result、restore flow 与 candidate admission metadata；CLI 只暴露 dry-run admission，不得直接写 local branch 或 Projection Workspace。 |
| `document_runtime` | `部分承载` | `apps/cli/src/server/handlers/document/`; `apps/web/src/runtime/document_client/`; `apps/web/src/editor/` | `docs/tasks/20_web_thin_client_ledger_migration.md` | OpenDoc、snapshot、history 与 edit intent；Web 端只作为 `document_client` adapter。 |
| `source_control_client` | `部分承载` | `apps/web/src/runtime/source_control_client/`; `apps/web/src/runtime/source_control_client/diff_session.rs`; `apps/web/src/hooks/use_core/callbacks_sc/`; `apps/web/src/hooks/use_core/callbacks_sc.rs`; `apps/web/src/hooks/use_core/callbacks_sc_scope.rs`; `apps/web/src/hooks/use_core/callbacks_sc_target.rs`; `apps/web/src/components/sidebar/source_control/` | `docs/tasks/19_repo_refactor_blueprint.md` | Browser source-control typed intent、diff session 与 readonly projection；不得拥有 stage/commit authority。 |
| `render_projection_runtime` | `部分承载` | `apps/web/src/runtime/rendering_client/`; `apps/web/src/editor/`; `apps/web/src/components/outline_render/`; `apps/web/js/chat_math_bootstrap.js`; `apps/web/js/chat_math.js`; `apps/web/index.html` | `docs/tasks/18_infra_runtime.md` | Projection-only rendering state、chat math pass 与 render hints；`index.html` 只允许保留过渡 bootstrap。 |
| `widget_bridge_runtime` | `部分承载` | `apps/web/src/runtime/rendering_client/`; `apps/web/js/web_bridge_registry.js`; `apps/web/js/extensions/`; `apps/web/src/editor/`; `apps/web/index.html` | `docs/tasks/18_infra_runtime.md` | Widget/editor/browser bridge 与 renderer extension，不拥有 authority；`window.*` 必须经 bridge registry 收敛。 |
| `outline_projection_runtime` | `部分承载` | `apps/web/src/components/outline_render/`; `apps/web/src/hooks/use_outline.rs` | `docs/tasks/18_infra_runtime.md` | Outline projection only；不得成为 document authority。 |
| `ui_shell` | `抽象分层` | `apps/web/src/components/`; `apps/desktop/src/shell/`; `apps/mobile/src/shell/` | `docs/tasks/18_infra_runtime.md` | View intent、layout shell、panel/focus/stacking 管理。 |
| `application_control` | `抽象分层` | `apps/web/src/hooks/use_core/`; `apps/web/src/runtime/session_client/`; `apps/web/src/runtime/scope_client/`; `apps/web/src/runtime/document_client/`; `apps/web/src/runtime/source_control_client/`; `apps/web/src/runtime/rendering_client/`; `apps/desktop/src/shell/`; `apps/mobile/src/shell/` | `docs/tasks/19_repo_refactor_blueprint.md` | Control 编排、typed intent routing 与 runtime 状态汇聚；Web `use_core` 必须降级为 composition root。 |
| `feature_runtime` | `抽象分层` | `apps/web/src/components/sidebar/source_control/`; `apps/web/src/components/command_palette/`; `apps/web/src/components/dashboard/` | `docs/tasks/19_repo_refactor_blueprint.md` | Feature-local state machines，不直接持有 authority。 |

## Notes

- `source_control_runtime -> Git mirror bridge` 仍以 `docs/plan/05_diff_logic.md#git-mirror-lifecycle` 为权威边界；本表只登记 runtime 当前承载。
- `session_client -> scope_client -> document_client -> pending_overlay_runtime -> write_confirmation_runtime` 是 Web write confirmation 主链；`pending_overlay_runtime` / `write_confirmation_runtime` 已有 typed contract，头段现在重启物理归带，旧 DEFER 裁定不再作为实现目标。
- `document_runtime -> render_projection_runtime -> widget_bridge_runtime / outline_projection_runtime` 必须保持 projection-only 边界。
- `ui_shell -> application_control -> feature_runtime` 当前是抽象分层，不应为了满足表格而创建空模块。
- cli server 结构中 `server/runtime/` 带已重启实施：startup assembly 必须收敛为 runtime parts；`repo_scope_sync_runtime` / `session_runtime` / `auth_gateway` 仍可按关切分离，但不能继续把 `start_server_with_options` 作为长期承载边界。
- crates/core 顶层 `authority/`/`projection/`/`scope/` 重组经 §8 核对 **DEFER**:蓝图「projection 与 authority 必须明确分层」的**实质要求在模块层已满足**(`authority_storage_runtime` / `projection_*_runtime` / `repo_scope_runtime` 均 `已收敛` 专用模块),剩纯目录提级而蓝图明示「不要求一次性改目录名」;强行提级会拆散 `ledger/manager/` 聚合。`watcher_runtime` 维持 `部分承载` 是契约 facade(`crates/core/src/watcher.rs`,`03_storage/watcher#watcher-contract`)vs `sync/watcher/` 实现的**有意分层**,非散落——依据见 `docs/report/core-toplevel-reorg-convergence-decision-2026-05-29.md`。

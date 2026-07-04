# external_changes.md - External Changes 操作流

## Metadata

- `Flow ID`: `flow.external-changes`
- `Domain`: `external-changes`
- `Related Feature Chapters`: `docs/features/07_diff_logic.md`, `docs/features/08_ui_design_03_mobile.md`
- `Related Acceptance Cases`: `DIFF-011`, `UI-MOB-024`

## Operations

### `op.external-changes.refresh`

- `Name`: `Refresh External Changes`
- `Surface`: `external-changes-panel`
- `Trigger`: 打开 External Changes / 外部修改，或用户触发 refresh
- `Preconditions`: 当前 repo scope 可读，未处于 repo/branch switching
- `Immediate Result`: 前端发送只读 refresh intent，服务端返回 pending external / staged external / confirmed ledger overlap 所需状态
- `Application Entry`: `apps/web/src/runtime/external_changes_client/mod.rs`, `apps/web/src/hooks/use_core/contexts/external_changes.rs`, `apps/cli/src/server/ws/route/source_control.rs`

### `op.external-changes.stage`

- `Name`: `Stage External Change`
- `Surface`: `external-changes-panel`
- `Trigger`: 点击单个 external change 的 `Stage`
- `Preconditions`: 条目位于 external unstaged 区域，write gate 未阻塞，且不与 confirmed ledger dirty 重叠
- `Immediate Result`: 前端发送 typed stage intent；服务端只迁移 External Changes staging，不写 ledger
- `Application Entry`: `apps/web/src/components/sidebar/external_changes/row.rs`, `apps/web/src/runtime/external_changes_client/mod.rs`, `apps/web/src/api/external_changes.rs`

### `op.external-changes.unstage`

- `Name`: `Unstage External Change`
- `Surface`: `external-changes-panel`
- `Trigger`: 点击 staged external change 的 `Unstage`
- `Preconditions`: 条目位于 staged external 区域，write gate 未阻塞
- `Immediate Result`: 条目回到 External Changes unstaged 区域
- `Application Entry`: `apps/web/src/components/sidebar/external_changes/row.rs`, `apps/web/src/runtime/external_changes_client/mod.rs`, `apps/web/src/api/external_changes.rs`

### `op.external-changes.discard`

- `Name`: `Discard External Change`
- `Surface`: `external-changes-panel`
- `Trigger`: 点击 external change 的 `Discard External Change`
- `Preconditions`: write gate 未阻塞
- `Immediate Result`: projection workspace 中对应路径恢复为当前 ledger projection；该 external change 从列表消失
- `Application Entry`: `apps/web/src/components/sidebar/external_changes/row.rs`, `apps/web/src/api/external_changes.rs`, `apps/cli/src/server/handlers/source_control/http_mutations/mod.rs`

### `op.external-changes.apply-to-ledger`

- `Name`: `Apply to Ledger`
- `Surface`: `external-changes-panel`
- `Trigger`: 点击 `Apply to Ledger` / `确认外部修改`
- `Preconditions`: staged external changes 非空，write gate 未阻塞，且 staged set 不与 confirmed ledger dirty 重叠
- `Immediate Result`: 服务端把 staged external changes 转换为 ledger facts，清空 External Changes staging；Source Control 后续显示对应 `Confirmed Ledger Changes`
- `Application Entry`: `crates/core/src/ledger/manager/commit_runtime.rs`, `apps/cli/src/server/handlers/source_control/service/write.rs`, `apps/web/src/runtime/external_changes_client/mod.rs`

### `op.external-changes.overlap-blocked`

- `Name`: `Overlap With Confirmed Ledger Changes`
- `Surface`: `external-changes-panel`
- `Trigger`: 外部修改与 confirmed ledger dirty 指向同一 `DocId` 或 canonical path
- `Preconditions`: 同一文档同时存在 external change 与 confirmed ledger dirty
- `Immediate Result`: UI 显示 `与已确认账本更改重叠` / `Overlaps confirmed ledger changes`，禁用普通 `Stage` 与 `Apply to Ledger`
- `Application Entry`: `crates/core/src/source_control/external_overlap.rs`, `apps/web/src/components/sidebar/external_changes/row.rs`

## Notes

- External Changes 可共享 UI primitive，但不能复用 Source Control commit/history/graph controller。
- `Apply to Ledger` 不是 `Commit`；它只写 ledger facts，不创建 commit anchor。
- Source Control 只展示 ledger/version-anchor 状态；External Changes 只展示投影文件夹偏差。

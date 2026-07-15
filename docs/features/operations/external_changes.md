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
- `Immediate Result`: 服务端单事务完成 staged/pending rows 与 DocId indexes 的迁移后，条目回到 External Changes unstaged 区域；目标漂移或写入失败时保持原 staged 状态
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
- `Immediate Result`: 服务端把 staged external changes 转换为 ledger facts，清空 External Changes staging，返回绑定 authority head 的 typed receipt，并为同 repo session 发布一次后端指定范围的 projection recovery；命中当前文档时通过 fresh Snapshot/History 收敛，Source Control 后续显示对应 `Confirmed Ledger Changes`
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
- Stage 固化用户确认时的 workspace 内容 hash；若文件在 Stage 后再次被外部修改，Apply 必须
  fail-closed 并要求重新 scan/stage，不能把未确认的新内容写入 ledger。
- Unstage 对调用前解析的完整 staged entry 做事务内重新解析与 exact compare；并发替换不得被误消费，也不得覆盖 watcher 已写入的同路径较新 pending 证据。
- Source Control 只展示 ledger/version-anchor 状态；External Changes 只展示投影文件夹偏差。

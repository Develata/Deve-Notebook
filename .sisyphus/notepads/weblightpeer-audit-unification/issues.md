# Issues - WebLightPeer Audit Unification

记录执行过程中遇到的阻塞、异常与待协调问题。

---

## [2026-03-06 T6a] `cargo check --package deve_web` 现存阻塞

- 当前 `cargo check --package deve_web` 失败于既有文件 `apps/web/src/components/desktop_layout.rs:48`、`apps/web/src/components/mobile_layout/content.rs:19`、`apps/web/src/components/mobile_layout/mod.rs:130`
- 失败原因均为 `use_core::types::CoreState` 缺少字段 `sync_banner`，与本次 `apps/web/src/storage/` 骨架改动无关
- 由于任务明确禁止修改 `apps/web/src/storage/` 以外文件，本次仅记录阻塞，待后续上游状态修复后再恢复包级构建验证

## [2026-03-06 sync_banner] 现状与新阻塞

- `apps/web/src/hooks/use_core/types.rs:38` 已存在 `pub sync_banner: Signal<Option<String>>`
- `apps/web/src/hooks/use_core/mod.rs:110` 已存在 `sync_banner: signals.sync_banner.into(),`
- 当前 `cargo check --package deve_web` 的新阻塞已转为 `crate::storage` 未导出：`apps/web/src/hooks/use_core/state.rs:7` 与 `apps/web/src/hooks/use_core/mod.rs:22`
- 因本次任务限制只允许触碰 `CoreState` 相关定义/初始化，未继续修复 `apps/web/src/lib.rs` 或 `apps/web/src/main.rs` 的模块导出缺口

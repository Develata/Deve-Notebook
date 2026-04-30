# Web Release Graph Warning Cleanup - 2026-04-30

## Scope

关闭 `trunk build --release` 中由 `apps/web/src/components/sidebar/source_control/graph_panel.rs` 触发的 `graph_state_attr` unused warning。

## Change

- 删除生产代码未使用的 `graph_state_attr` helper。
- 保留真实渲染路径使用的 `graph_loaded_state_attr`。
- 调整 graph panel 测试，使其直接覆盖 loaded/empty 两种 `data-deve-graph-state` 值来源。

该变更不改变 Graph summary panel 的 UI 行为：idle/loading/error/blocked/local-only/degraded 仍由 `graph_message` 显式传入 state，loaded/empty 仍由 `graph_loaded_state_attr` 派生。

## Verification

- `cargo fmt --check`
- `cargo test -p deve_web graph_panel -- --nocapture`
- `env NO_COLOR=true trunk build --release`

## Notes

- 直接执行 `trunk build --release` 会因当前环境的 `NO_COLOR=1` 触发 Trunk 参数解析错误；显式 `NO_COLOR=true` 是当前稳定运行方式。
- release build 已不再输出 `graph_state_attr` warning。
- build 仍输出 `Browserslist: caniuse-lite is outdated` 提示；这是独立前端依赖数据 freshness 问题，已排入下一队列，不混入本批次。


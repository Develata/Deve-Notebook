# Feature Acceptance Gap Scan - 2026-05-12 04

本报告记录 `I18N localized formatting browser smoke` 之后的 feature / acceptance / code 交叉扫描。`docs/plan/` 仍是唯一权威；本文件只记录执行结果与下一步队列。

## Scope

- `docs/features/operation-coverage.md`
- `docs/features/operations/`
- `docs/acceptance-cases/`
- `docs/acceptance-bindings.tsv`
- `apps/web/src/components/diff_view/`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-ui-desktop-baseline.sh`
- `scripts/check-mobile-baseline.sh`

## Verification Snapshot

已运行：

- `scripts/check-acceptance-bindings.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/check-source-control-baseline.sh`
- `scripts/check-ui-desktop-baseline.sh`
- `scripts/check-mobile-baseline.sh`
- `cargo test -p deve_web diff_first_viewport -- --nocapture`
- `cargo test -p deve_web desktop_diff_scroll -- --nocapture`
- `cargo test -p deve_web mobile_diff -- --nocapture`
- `rg -n "Diff:|Read Only|Preview Diff|Close Diff View" apps/web/src/components/diff_view`
- `scripts/plan-coverage.sh`
- `git diff --check`

结果：

- acceptance binding: `68 automated / 67 feature / 49 manual / 0 unbound`
- architecture registry: `72 flows, 0 active drift`
- plan coverage blocking violations: `0`

## Findings

### F1. P0 Gap Not Found

结果：

- Acceptance binding 没有 unbound case。
- 最近 runtime / search / rendering / source-control / release / i18n smoke 均已形成报告。
- 本轮未发现必须先修的 plan/code 矛盾。

### F2. UI-DIFF Is The Largest User-Visible Closure Gap

问题：

- `UI-DIFF-002..018` 覆盖移动 diff 编辑 debounce、hunk navigation、fold、context lines、anchor restore、cache badge、cache ratio、algorithm label、repo-scope cache isolation 等用户可见行为。
- 当前 `scripts/check-source-control-baseline.sh` 主要定型 `UI-DIFF-001` first viewport。
- 代码侧已有 `diff_view` header、fold controls、viewport、cache metrics、navigation 与 word diff 结构，但多个 acceptance case 仍停留在 manual Chrome 断言。

已验证现状：

- `diff_first_viewport` 测试通过。
- `desktop_diff_scroll` 测试通过。
- `mobile_diff` 测试通过。
- Diff 组件目录未残留 `Diff:`、`Read Only`、`Preview Diff`、`Close Diff View` 这类硬编码文案。

下一批应先做小而硬的 UI-DIFF closure：

- 修正 `docs/acceptance-bindings.tsv` 中 `UI-DIFF-002..018` 的语义备注漂移。
- 给已有 diff behavior 增加最小可跑的 unit / baseline guard。
- 对仍只能靠浏览器确认的行为保留 Chrome MCP smoke 口径，不把 smoke 描述成代码实现。

### F3. Storage / Repo Acceptance Contains Stale CLI Surface

问题：

- `docs/acceptance-cases/07_storage_repo.md` 仍包含 `deve repo create`、`deve db inspect`、`deve doc edit`、`deve api call`、`deve path normalize` 等命令。
- 当前 CLI surface 与 `docs/plan/12_commands.md` baseline 不包含这些命令。

判断：

- 这是 acceptance 文档漂移，不是要求立即新增伪 CLI。
- 下一批之后应把这些 case 改写到现有 CLI、HTTP/API 测试或 manual evidence，避免 docs/acceptance 反向塑造 command surface。

### F4. Browser-Only Gaps Remain, But They Are Not First

仍需后续补齐：

- `WEBWRITE-FEAT-01/02/03` pending navigation / reject 的真实浏览器闭环。
- `DIFF-003/004` merge conflict UI 的真实浏览器闭环。
- `NET-013 / AUTH-011` logout / session expired 与 reconnect 分离的真实浏览器闭环。
- `SEARCH-002` disabled / low-spec fail-closed UI 的浏览器 smoke。
- `AI-001` 正向 Native AI browser smoke 依赖可用测试 provider 或 mock provider，当前缺 API key 错误路径不能等价替代。

## Next Execution Queue

1. UI Diff acceptance closure：修正 `acceptance-bindings.tsv` 的 `UI-DIFF-*` 语义漂移，并为现有 diff behavior 补最小自动 guard / baseline。
2. Storage / Repo acceptance command drift audit：按当前 plan command surface 改写 `07_storage_repo.md` 的过时 CLI 步骤，不新增 plan 未列出的伪命令。
3. WebWrite pending navigation browser smoke：隔离后端 + Chrome MCP 验证 pending modal、Stay、确认离开与 Reject 后不永久 pending。

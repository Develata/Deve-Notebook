# Mainline Implementation Gap Scan - 2026-05-12

本报告记录当前主线实现缺口扫描。`docs/plan/` 仍是唯一权威；本文件只作为执行队列输入。

## Scope

- `docs/plan/`
- `docs/features/operations/`
- `docs/acceptance-cases/`
- `docs/overview/architecture-diff.md`
- 当前代码与 smoke scripts

旧 `gap-*-2026-04-08.md` 只作 forensic input；其中已过时断言不得直接转成 TODO。

## Verification Snapshot

已运行：

- `scripts/plan-coverage.sh`
- `scripts/check-search-baseline.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-native-packaging-gate.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/smoke-runtime-happy-path.sh`
- `scripts/smoke-runtime-recovery-path.sh`
- `cargo test -p deve_cli search -- --nocapture`
- `cargo test -p deve_cli --features search search -- --nocapture`

结果：

- plan coverage blocking violations: 0
- architecture registry: 72 flows, 0 active drift
- runtime happy-path smoke: pass
- runtime recovery smoke: pass
- search feature-off fail-closed path: pass
- search feature-on repo-scoped baseline scan: pass
- native/desktop/mobile packaging gates: pass

## Findings

### G1. Browser Runtime E2E Is The Next Real User-Facing Gap

当前 runtime smoke 覆盖 WS route、scope gate、write gate、reconnect bootstrap 与 degraded recovery 的自动化单测链路，但还没有在本轮 Phase 2 架构迁移后重新执行完整浏览器链路：

- isolated `serve --dev`
- 浏览器加载 Web shell
- 登录或 dev session bootstrap
- repo ready
- create/open/edit/save/reload/reconnect
- 页面不进入 stale scope、unsupported protocol version 或 disconnected lockout

这不是 plan/code 矛盾；这是 release 前必须补齐的实机验收缺口。

### G2. Search Contract Is Implemented, But Browser Smoke Remains Manual

Search 当前状态：

- `search` feature 未启用时返回结构化错误。
- LowSpec/runtime disabled 路径 fail closed。
- feature-on 时服务端执行 repo-scoped baseline scan。
- SearchResults 携带 `request_id/repo_id/branch/scope_nonce`。
- 前端 stale request/scope gate 已有测试覆盖。

剩余缺口是 Chrome MCP 手工 smoke：在真实 Web shell 中输入 `?note`，确认结果显示、选择结果打开文档，并确认底部状态保持 Ready。

### G3. Feature Operation Mapping Has One Stale Path

`docs/features/operations/search_query.md` 中 `op.search.submit` 的 Application Entry 写作：

- `apps/web/src/hooks/use_core/callbacks_misc.rs`

当前代码实际路径是：

- `apps/web/src/hooks/use_core/callbacks/misc.rs`

这是 features 层路径映射漂移，不影响 runtime，但会误导后续审查与 Chrome MCP 复现步骤。

### G4. Platform Packaging Remains Correctly Gate-Closed

Desktop/mobile 当前仍是 shell/scaffold 与 native-adapter contract，不是完整 macOS/Windows/Android/iOS 发布包。

已验证：

- 默认 workspace 不引入真实 Tauri dependency。
- packaging gate 关闭。
- native adapter 不能获得 ledger/vault/source-control/search/`.git`/`.notegit` authority。

因此 platform packaging 不应抢占下一批主线实现优先级。

## Next Execution Queue

1. 修复 G3 的 features 层路径漂移。
2. 执行 Chrome MCP browser runtime E2E：isolated `serve --dev` + Web shell ready/edit/reload/reconnect。
3. 执行 Chrome MCP search smoke：`?note` 搜索、结果展示、打开文档、Ready 状态确认。
4. 若 browser E2E 暴露协议版本、auth/session、scope 或 disconnected 问题，优先修复 runtime；否则进入下一轮 feature/acceptance gap scan。


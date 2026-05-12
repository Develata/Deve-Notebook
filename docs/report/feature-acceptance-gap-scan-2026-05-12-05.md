# Feature Acceptance Gap Scan - 2026-05-12 05

本报告记录 `WebWrite pending navigation smoke` 之后的 feature / acceptance / code 交叉扫描。`docs/plan/` 仍是唯一权威；本文件只记录执行队列输入。

## Scope

- `docs/report/next-tasks.md`
- `docs/report/*2026-05-12*.md`
- `docs/features/operations/`
- `docs/acceptance-cases/`
- `docs/acceptance-bindings.tsv`
- current smoke / guard scripts

旧 `gap-*-2026-04-08.md` 只作 forensic input；其中已过时断言不得直接转成 TODO。

## Verification Snapshot

已运行：

- `scripts/check-acceptance-bindings.sh`
- `scripts/check-feature-operation-paths.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/plan-coverage.sh`
- `git status --short`

结果：

- acceptance binding: `93 automated / 62 feature / 29 manual / 0 unbound`
- feature operation path check: pass
- architecture registry: `72 flows, 0 active drift`
- plan coverage blocking violations: `0`
- working tree before this report: clean

## Findings

### F1. P0 Gap Not Found

结果：

- `docs/report/next-tasks.md` 当前队列已清空。
- `plan-coverage` 没有 blocking violation。
- acceptance cases 没有 unbound case。
- operation path 与 architecture registry 均无 active drift。

判断：

- 当前不应凭空开启大规模重构或新平台分支。
- 下一批应继续补用户可见 smoke / acceptance 闭环。

### F2. UI Diff Browser Interaction Still Lacks Real Smoke Evidence

已完成：

- `UI-DIFF-*` binding 语义漂移已修。
- `diff_view` 已有大量 unit / baseline guard。
- `UI Diff acceptance closure` 已记录 `73` 个 diff 相关测试通过。

仍缺：

- hunk button click。
- keyboard navigation。
- fold click。
- context line selector。
- cache badge UI。
- mobile edit debounce 的真实 viewport 行为。

判断：

- 这是当前最直接的用户可见验收缺口。
- 先做 Chrome MCP smoke；若 smoke 暴露 UI/runtime bug，再修代码。

### F3. Search Disabled / Low-Spec Fail-Closed Needs Browser Smoke

已完成：

- Search 正向 `?note` browser path 已通过。
- feature-on/off 与 baseline guard 已存在。

仍缺：

- `SEARCH-002` disabled / low-spec fail-closed UI 的浏览器证据。

判断：

- 这不要求新搜索能力，只要求证明低配或禁用路径不会给用户展示错误可用性。

### F4. Auth Logout / Session Expired Must Stay Separate From Reconnect

仍缺：

- `NET-013 / AUTH-011` 的真实浏览器闭环。
- 需要验证 logout/session-expired 是 auth state，不应被误渲染为普通 disconnected/reconnecting。

判断：

- 该项影响实际使用时的 lockout 诊断，优先级高于 AI 正向 smoke。

### F5. Native AI Positive Smoke Depends On Test Provider

状态：

- AI backend capability / fail-closed / default-off 已有自动化覆盖。
- 当前没有稳定测试 provider 或 mock provider 能在无外部 API key 时证明正向 Native AI browser flow。

判断：

- 不应把真实 API key 当验收前提。
- 后续若要做 `AI-001` 正向 smoke，应先补最小 test provider / mock provider，再跑浏览器正向路径。

## Next Execution Queue

1. UI Diff browser interaction smoke：隔离后端 + Chrome MCP 验证 hunk click、keyboard navigation、fold click、context select、cache badge 与 mobile edit debounce。
2. Search disabled / low-spec fail-closed browser smoke：隔离后端 + low-spec 或 search-disabled 配置，验证 Search UI 不宣称不可用能力。
3. Auth logout / session-expired browser smoke：隔离后端 + Chrome MCP 验证 logout/session-expired 与普通 reconnect/disconnected 状态分离。
4. Native AI positive smoke provider preflight：确认是否已有无外部 key 的 test provider；没有则先设计最小 mock provider，不直接要求真实 API key。

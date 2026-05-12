# Mainline gap selection after smoke closure - 2026-05-12

## Scope

- 唯一真源：`docs/plan/`
- 输入：
  - `docs/report/mainline-gap-scan-2026-05-12.md`
  - `docs/report/feature-acceptance-gap-scan-2026-05-12-05.md`
  - 2026-05-12 已完成 smoke reports
  - `docs/features/`
  - `docs/acceptance-cases/`
  - 当前代码与 guard scripts

本报告只选择下一批执行队列，不修改 plan。

## Verification Snapshot

已运行：

- `scripts/check-acceptance-bindings.sh`
- `scripts/check-architecture-registry.sh`
- `scripts/check-ai-baseline.sh`
- `scripts/check-rendering-baseline.sh`

结果：

- acceptance binding: `93 automated / 62 feature / 29 manual / 0 unbound`
- architecture registry: `72 flows, 0 active drift`
- AI baseline: pass
- rendering baseline: pass

## Closed Queue Since Last Selection

以下 `feature-acceptance-gap-scan-2026-05-12-05.md` 中的 active queue 已闭合：

- UI Diff browser interaction smoke：已完成 `ui-diff-browser-interaction-smoke-2026-05-12.md`。
- Search disabled / low-spec fail-closed smoke：已完成 `search-disabled-fail-closed-smoke-2026-05-12.md`。
- Auth logout / session-expired smoke：已完成 `auth-session-expired-browser-smoke-2026-05-12.md`。
- Native AI positive smoke provider preflight：已完成 `native-ai-positive-smoke-2026-05-12.md`，并修复 `ai-chat` Rhai prompt 常量作用域 bug。

## Selection Rules

- 不把 `future` / gate-closed 能力转成当前实现任务。
- 不以抽象重构替代用户可见验收。
- 优先选择 Web/server 主线中已实现但缺少真实浏览器闭环的能力。
- 若 Chrome smoke 暴露真实 bug，先修 bug，再更新 report / guard。

## Not Selected

- Desktop / mobile 原生 packaging：仍受 native-packaging gate 约束；当前是 shell scaffold 与 adapter contract，不应抢占 Web/server 主线。
- Graph 高性能 renderer：当前权威边界是只读 projection summary；Canvas/d3-force/Pixi renderer 仍为 future。
- Server-backed Settings API / GUI 持久化：仍为 future；当前稳定入口是 `config.toml` 与 `deve config print/set`。
- Git mirror 可执行 Web repair：plan 明确 Web repair 当前只能是 CLI-only notice / readonly review，不应新增后台 Git writer。
- MCP runtime：产品方向已退役，不进入任何执行队列。

## Selected Gaps

### G1. AI BUILD controlled Apply browser smoke

优先级：P1

权威来源：

- `docs/plan/10_ai_agent.md#native-ai-chat-runtime`
- `docs/features/10_ai_agent.md`
- `docs/features/operations/ai_chat.md`
- `docs/acceptance-cases/10_plugins.md#AI-003`

现状：

- Native AI 正向 text response 已用本地 SSE mock 通过。
- `chat_apply`、scope nonce、BUILD-only Apply button 有单元测试与 baseline guard。
- 尚未用真实浏览器验证 `/build` → assistant code block → `Apply` → `ClientMessage::Edit` → 当前 Markdown 改变的完整链路。

验收口径：

- 不依赖真实外部 API key，继续使用本地 OpenAI-compatible SSE mock。
- mock response 必须包含 Markdown code block。
- `/build` 本身不发送 plugin call。
- Apply 只在 BUILD 模式可见。
- 点击 Apply 后当前文档内容改变，并通过既有 write gate / scope nonce / `ClientMessage::Edit` 路径提交。
- Console 无 error / warn；不得出现 `mcp`、`skill`、`spawn subprocess`、`shell`。

### G2. Merge conflict UI browser smoke

优先级：P1

权威来源：

- `docs/plan/07_diff_logic.md`
- `docs/features/07_diff_logic.md`
- `docs/features/operations/sc_merge_peer.md`
- `docs/features/operations/sc_resolve_conflict.md`
- `docs/acceptance-cases/04_diff.md#DIFF-003`
- `docs/acceptance-cases/04_diff.md#DIFF-004`

现状：

- Merge conflict server / protocol / resolution tests 已存在。
- Source Control browser smoke 已覆盖 normal pending、stage、unstage、commit、refresh、reload。
- 尚未用真实浏览器验证 `MergeConflict` UI 的三种 action。

验收口径：

- 优先复用现有 test harness 生成 conflict；若真实浏览器 fixture 成本过高，先补最小 fixture endpoint 或 deterministic CLI setup，不能用手写 DOM 假数据替代 server message。
- UI 必须显示 `accept-current`、`accept-incoming`、`accept-both`。
- 点击 action 必须发送 `ResolveMergeConflict`，并携带 `doc_id`、`action`、`result_content`、`scope_nonce`。
- remote/spectator/read-only scope 不得伪装可写。

### G3. Rendering interaction spot smoke

优先级：P2

权威来源：

- `docs/plan/03_rendering.md`
- `docs/features/03_rendering.md`
- `docs/acceptance-cases/03_rendering.md`

现状：

- 已完成 checkbox writeback、Math projection、Ready 后 search path。
- 尚未在本轮 smoke 中覆盖 code toolbar、Ctrl/Cmd link activation、Outline navigation、Mermaid projection、nested rendering。

验收口径：

- 隔离数据根 + Chrome MCP。
- 按小批次覆盖，不把 rendering 扩展成富文本 authority。
- 若 Mermaid / link activation 受浏览器安全限制，记录限制并补最小可自动化 guard。

### G4. Settings / Extensions reserved UI browser smoke

优先级：P2

权威来源：

- `docs/plan/10_ai_agent.md`
- `docs/plan/13_settings.md`
- `docs/plan/17_plugins.md`
- `docs/features/operations/settings_runtime_feedback.md`
- `docs/acceptance-cases/10_plugins.md#PLUG-001`
- `docs/acceptance-cases/10_plugins.md#PLUG-002`
- `docs/acceptance-cases/15_settings_operation_refs.md`

现状：

- Settings / AI / Extensions 的 disabled-state helper 和 baseline guard 已存在。
- 缺少当前浏览器下 Extensions / Settings reserved UI 的最小可见性 smoke。

验收口径：

- Trusted CLI 显示 default-off / disabled reason。
- Calculation Runtime 显示 planned / disabled。
- reserved controls 暴露可机检 disabled marker / `aria-disabled`。
- 不新增 server-backed Settings API，不引入执行型 plugin runtime。

## Next Execution Queue

1. AI BUILD controlled Apply browser smoke。
2. Merge conflict UI browser smoke。
3. Rendering interaction spot smoke。
4. Settings / Extensions reserved UI browser smoke。

第一项优先，因为它直接闭合 `AI-003`，并且复用刚建立的 Native AI local SSE mock，不需要引入新产品代码。

# 剩余工作分支规划 (Next Tasks — Branch Decomposition)

> **生成日期**: 2026-02-28
> **更新日期**: 2026-04-28
> **当前权威队列**: 以 “Current Execution Queue” 为准；下方旧 Branch A-E 仅保留为历史分支拆解参考。

## Current Execution Queue

本队列按当前项目方向重新排序：先补 P0 根基，再补 P1 产品可用主线，然后把 Desktop/Mobile 与 Graph 提上日程，最后处理 P2 运维与体验补强。AI Chat 只保持最小可用；MCP 不进入产品实现路线，相关需求由 Skills + 受控 CLI 工具调用替代。

| 顺序 | TODO | 优先级 | 范围 | 验收口径 |
|:-----|:-----|:------|:-----|:---------|
| 1 | P0 Authority / Repo Health / Repair | P0 | `crates/core/src/ledger/`, `crates/core/src/sync/`, `apps/cli/src/commands/{node_check,repair}.rs` | authority 不被 projection 反写污染；degraded repo 显式可观测；repair/node-check fail-closed |
| 2 | P0 WS / Repo-Scoped Protocol / Write Readiness | P0 | `crates/core/src/protocol/`, `apps/cli/src/server/ws/`, `apps/web/src/hooks/use_core/effects/` | `repo_id / branch / scope_nonce / writer_ready` 全路径稳定；结构化错误覆盖 auth/ws/write gate |
| 3 | P0 Source Control Core Path | P0 | `crates/core/src/source_control/`, `apps/cli/src/server/handlers/source_control/`, `apps/web/src/hooks/use_core/callbacks_sc*` | Stage / Commit / Diff / Merge / Discard 走 Node-first、doc_id-first、fail-closed；server handler 不散落 authority 逻辑 |
| 4 | P1 Search Baseline | P1 | `crates/core/src/search/`, `apps/cli/src/server/handlers/search.rs`, `apps/web/src/components/search_box/` | 当前 repo-scoped baseline search 稳定；低配模式可禁用；Tantivy 增量索引仍为 future optimization |
| 5 | P1 Settings Current Boundary | P1 | `crates/core/src/config.rs`, `apps/cli/src/commands/config.rs`, `apps/web/src/components/settings*` | 继续只承诺 `config.toml + config print/set + UI runtime feedback`；server-backed Settings API 不进入当前验收 |
| 6 | P1 Native AI Chat Minimum | P1 | `apps/cli/src/server/ai_chat/`, `apps/web/src/components/chat/`, `apps/web/src/api/ai_backend.rs` | 保持“读当前 Markdown + chat + PLAN/BUILD 最小模式”；不默认启用 MCP / Skills / Source Control 写入 |
| 7 | P3-10 Desktop / Mobile Native Track | P3-10 | `docs/plan/08_ui_design_02_desktop.md`, `docs/plan/08_ui_design_03_mobile.md`, future Tauri shell | Desktop/Mobile 逐步进入路线图；先明确 adapter、embedded service、offline/readiness 边界，再实现 native packaging |
| 8 | P3-13 Graph / Knowledge Visualization | P3-13 | future graph data model, `docs/plan/14_tech_stack.md` | 等 repo/search/metadata 数据稳定后再实现；不得反向污染 ledger authority |
| 9 | P2 Runtime / Release / UI Debt | P2 | `.github/`, `scripts/`, `docs/plan/15_release.md`, `apps/web/src/hooks/use_core/`, `apps/web/src/i18n/` | runtime observability、release smoke、CoreSignals 收敛、i18n allowlist debt 逐步清理 |

### Current Status Notes

- 2026-04-28: P1 Search Baseline 已补前端 SearchResults 生命周期闭环：结果必须通过 `request_id / repo_id / branch / scope_nonce` gate，接受后清空 pending search request，避免后续无关 `ProtocolError` 被误归类为 search notice；默认无 search feature 与 `--features search` 后端路径均已通过定向测试。
- 2026-04-28: P1 Settings Current Boundary 已补 UI 边界提示：Settings 面板明确说明当前只提供运行时/本地 UI 反馈，持久运行时配置仍通过 `deve config set` 写入 `config.toml`；baseline 脚本已检查该边界，server-backed Settings API 仍不进入当前验收。
- 2026-04-28: P1 Native AI Chat Minimum 已补当前 Markdown 上下文与 PLAN/BUILD prompt 边界：前端向 Native AI Chat 传递有界当前正文，ai-chat 插件将当前文件、正文、selection、模式写入 system prompt；默认仍不开放 workspace/source-control/shell/MCP/skill 执行能力。
- 2026-04-28: P3-10 Desktop/Mobile Native Track 已补当前 native adapter 边界：Desktop/Mobile docs 明确 Web responsive shell 是当前可验收映射，Tauri packaging 仍为 future；adapter 第一阶段仅负责内嵌服务、endpoint/session 注入与 readiness/offline 事件，不得重定义 core/server authority。
- 2026-04-28: P3-13 Graph / Knowledge Visualization 已补 core 只读 projection baseline：`deve_core::graph` 从 repo-scoped docs 派生节点、已解析边与未解析链接，支持 wikilink / markdown `.md` link 与路径归一化；不读取/写入 ledger、metadata、workspace、search 或 source-control 状态，d3/Pixi 渲染仍是 future。
- 2026-04-28: P2 Runtime / Release / UI Debt 已把 native/graph 新基线纳入 dev runbook、release workflow 与 release baseline 检查，新增 `scripts/check-graph-baseline.sh` 防止 graph projection 漂移成 ledger/workspace authority path。
- 2026-04-28: P2 Runtime / Release / UI Debt 已完成一批小范围 i18n 收口：Dashboard repo 摘要、Activity/Mobile Pin/Unpin title、Editor outline/spectator 文案、Source Control no-repo notice 均改为 `t::*` facade，并补对应 i18n 单测。
- 2026-04-28: P2 Runtime / Release / UI Debt 已加固 `scripts/plan-coverage.sh` i18n 回归门禁：对已迁移的英文 UI 文案做精确回归扫描；同时修正 graph plan_ref anchor 与 Source Control status notice 的剩余 i18n 漏点，使 plan coverage 阻塞项回到 0。
- 2026-04-28: P2 Runtime / Release / UI Debt 已把 Source Control 错误提示、历史记录提示、counterpart badge 与 diff empty-state 文案集中迁入 `t::source_control::*`，组件侧 i18n allowlist debt 从 22 降到 1；剩余 `"中文"` 为语言选择器自指标签。
- 2026-04-28: P2 Runtime / Release / UI Debt 已把 Settings 语言选择器的自指语言标签也迁入 `t::settings::*`，`scripts/plan-coverage.sh` 的 i18n allowlisted debt 降到 0。
- 2026-04-28: P2 Runtime / Release / UI Debt 已补 `11_i18n.md` 稳定 anchors，并给 `apps/web/src/i18n/**` 当前 facade 模块补齐 plan_ref；`scripts/plan-coverage.sh --summary-missing-plan-ref` 中 i18n 目录 missing plan_ref 从 17 降到 0。
- 2026-04-28: P2 Runtime / Release / UI Debt 已补 `apps/web/src/api` WebSocket/API runtime 小批次 plan_ref，并在 `05_network.md`、`15_release.md`、`16_web_thin_client_ledger.md` 补稳定 anchors；全仓 missing plan_ref 从 622 降到 612。
- 2026-04-28: P2 Runtime / Release / UI Debt 已补 `apps/web/src/shortcuts` 与 `apps/web/src/storage` plan_ref，并同步 `04_storage.md`、`08_ui_design_01_web.md`、`12_commands.md`、`13_settings.md` 与 `docs/plan/AGENTS.md` 的稳定 anchors；下一轮继续从低耦合 Web runtime/support 模块向高耦合 hooks/components 收敛。
- 2026-04-28: P2 Runtime / Release / UI Debt 已补 `apps/web/src/utils` plan_ref，并把 Source Control history 的相对时间文案迁到 `i18n::time` facade，避免 util 层继续生成固定中文 UI 文案；Markdown renderer 绑定到渲染白名单 anchor。
- 2026-04-28: P2 Runtime / Release / UI Debt 已补 `crates/core/src/utils` plan_ref：路径规范化绑定 `04_storage#internal-path-normalization`，`.notegit` helpers 绑定 repo runtime layout，纯哈希/模块入口标记为 infra。
- 2026-04-28: P2 Runtime / Release / UI Debt 已补 core 小模块 plan_ref：`models` 绑定 facts partition，`state` 绑定 document authority / UTF-16 runtime，`search` 绑定 feature-gated search baseline，`skill` 绑定 MCP 退役后的 Skills + 受控 CLI 扩展边界；`lib.rs/error.rs` 标记为 infra。
- 2026-04-28: P2 Runtime / Release / UI Debt 已补 CLI/Web 顶层入口 plan_ref：CLI `main/dispatch/admin_api/dump_support` 绑定 command/diagnostic/export 边界，Web `main.rs` 绑定 single-binary Web shell 与 WS runtime 入口。
- 2026-04-28: P2 Runtime / Release / UI Debt 已补剩余 core/app 单点入口 plan_ref：`config` 绑定 Settings 与 Trusted CLI fallback，`context` 绑定 plugin-host 能力边界，`vfs` 绑定 watcher inode 抽象，`mock_divergence` 绑定 repo scope/source-control 测试边界。
- 2026-04-28: P2 Runtime / Release / UI Debt 已补 `crates/core/src/protocol` 与 `crates/core/src/security` plan_ref：协议消息/结构化错误绑定 WS runtime 与 Web thin-client 写意图，HTTP auth 类型绑定 auth endpoint/session/unauthorized 合同，安全 key/cipher/storage 绑定 repo runtime layout 与 sync runtime，纯哈希原语标记为 infra。

### MCP Direction

MCP 直接不做。当前判断是：MCP 趋势性不足，且主要用途可被 “Skills 调用受控 CLI 工具 / Trusted CLI path” 替代。文档中若保留 MCP 提及，只作为“为什么不做”的历史记录；不得把 MCP 解读为当前 TODO、future TODO、默认 AI 能力或插件平台方向。

## Legacy Branch Overview

以下内容是 2026-02-28 的旧分支拆解，保留用于追溯，不再代表当前执行顺序。

| 分支 | 优先级 | 预估 | 涉及 crate | 冲突风险 |
|:-----|:------|:-----|:----------|:--------|
| **A** `feat/css-design-tokens` | P2 | ~2 周 | `apps/web` | 高 |
| **B** `feat/server-dashboard` | P3 | ~1 周 | `apps/cli` + `apps/web` + `crates/core` | 低 |
| **C** `feat/e2ee-client` | P3 | ~3-5 天 | `apps/web` + `crates/core` | 低 |
| **D** `feat/plugin-system` | P3 | ~2 周 | `crates/core/plugin` + `apps/web/chat` | 低 |
| **E** `docs/progress-sync` | P3 | ~1 天 | 纯 Markdown | 无 |

---

## Branch A: `feat/css-design-tokens`

> 三项合并为一个分支：GAP-4 + 审计 #21 + 审计 #20。
> 原因：三者全部修改 `apps/web/src/components/**/*.rs`，分开做会产生严重冲突。

### A.1 组件目录规范化 (审计 #20)

**目标**: 将松散的顶层 `.rs` 文件重构为子目录结构。

**待处理文件** (当前为单文件, 应拆分为 `<name>/mod.rs`):
- `activity_bar.rs`
- `bottom_bar.rs`
- `header.rs`
- `dropdown.rs`
- `spectator_overlay.rs`
- `disconnect_overlay.rs`
- `merge_modal.rs` / `merge_modal_slot.rs` → 合并为 `merge_modal/`
- `merge_panel.rs`
- `settings.rs`
- `playback.rs`
- `outline.rs`
- `sidebar_menu.rs`
- `layout_context.rs`

**执行规则**:
- 重构后每个文件应保持职责内聚；超过 250 行需复查，超过 500 行需拆分或说明例外
- `mod.rs` 仅做 pub re-export
- 每步完成后 `cargo check --package deve_web` 验证

### A.2 lucide-leptos 图标迁移 (审计 #21)

**目标**: 替换 30+ 处内联 `<svg>` 硬编码为 `lucide-leptos` 组件调用。

**涉及文件** (含内联 SVG):
- `source_control/unstaged_section.rs` (3 处)
- `source_control/staged_section.rs` (2 处)
- `source_control/repositories.rs` (6 处)
- `source_control/mod.rs` (7 处)
- `source_control/history.rs` (1 处)
- `source_control/commit.rs` (2 处)
- `source_control/change_item.rs` (5 处)
- `sidebar/repo_switcher.rs` (1 处)
- `sidebar/extensions.rs` (1 处)
- `sidebar/explorer.rs` (1 处)
- `sidebar/components.rs` (1 处)
- `editor/mod.rs` (1 处)
- `spectator_overlay.rs` (1 处)

**操作**:
1. 在 `apps/web/Cargo.toml` 添加 `lucide-leptos` 依赖
2. 逐文件替换 `<svg>...</svg>` → `<ChevronRight />`, `<Plus />` 等语义化组件
3. 验证外观不变

### A.3 CSS Design Token 迁移 (GAP-4)

**目标**: 将 Tailwind 硬编码颜色类替换为 CSS 变量引用，实现主题切换。

**参考规范**: `deve-note plan/08_ui_design.md` §2.1

**步骤**:
1. **补全 `_variables.css`**: 新增 Plan 要求但尚未定义的变量:
   - `--bg-app`, `--bg-sidebar`, `--bg-editor`
   - `--fg-primary`, `--fg-secondary`, `--fg-muted`
   - `--border-default`, `--accent-primary`
   - 以及对应的 `.dark` 覆盖
2. **配置 `tailwind.config.js`**: 将 CSS 变量映射为 Tailwind 工具类:
   ```js
   colors: {
     'bg-app': 'var(--bg-app)',
     'fg-primary': 'var(--fg-primary)',
     // ...
   }
   ```
3. **逐组件替换**: `bg-white` → `bg-bg-app`, `text-gray-700` → `text-fg-primary` 等
4. **验证**: 切换 `.dark` class 确认暗色模式生效

**执行顺序**: A.1 → A.2 → A.3 (有严格先后依赖)

---

## Branch B: `feat/server-dashboard`

> GAP-6: 服务器仪表盘 — 规范来源: `08_ui_design_01_web.md` §2

### B.1 后端 Metrics 采集

**目标**: 新增 SystemMetrics 结构体和 `/api/metrics` 端点。

**涉及文件**:
- `crates/core/src/protocol.rs` — 新增 `ServerMessage::SystemMetrics` 变体
- `apps/cli/src/server/` — 新增 `metrics.rs`

**Metrics 字段** (来自 Plan):
```rust
struct SystemMetrics {
    cpu_usage_percent: f32,
    memory_used_mb: u64,
    active_connections: u32,
    ops_processed: u64,
}
```

**约束**: 768 MB 内存环境, 禁止常驻采集线程。使用 `/proc/stat` 解析或 `sysinfo` 轻量初始化。

### B.2 WebSocket 推送

**目标**: 服务器定时 5s 推送 `SystemMetrics` 到已认证客户端。

**涉及文件**:
- `apps/cli/src/server/ws/` — 新增推送逻辑
- 复用现有 WS broadcast 机制

### B.3 前端 Dashboard 组件

**目标**: 根路径 `/` 无 DocId 时渲染 Dashboard。

**新建文件**: `apps/web/src/components/dashboard/`
- `mod.rs` — 组件入口
- `health_card.rs` — CPU/RAM/Uptime 卡片 (Polling 5s)
- `sync_card.rs` — Connected Peers, Ops Queue (WS Push)
- `storage_card.rs` — DB Size, Document Count (On Load)
- `actions_card.rs` — `[New Doc]` `[Sync Now]` 按钮

**涉及修改文件**:
- `apps/web/src/app.rs` — 路由添加 Dashboard

**约束**:
- Dashboard 数据 MUST NOT 持久化到 IndexedDB (RAM-only)
- 需 JWT 认证保护 (auth middleware 已就绪)
- Web Socket断开时 Metrics 冻结 + "Disconnected" 提示

---

## Branch C: `feat/e2ee-client`

> GAP-5: E2EE 客户端密钥交换

### C.1 密钥交换协议

**目标**: WebSocket 握手后增加 `KeyExchange` 消息类型。

**涉及文件**:
- `crates/core/src/protocol.rs` — 新增 `ServerMessage::KeyExchange { encrypted_repo_key }` 和 `ClientMessage::RequestKey { repo_id }`
- `apps/cli/src/server/handlers/` — 密钥分发逻辑: 用客户端 Ed25519 公钥加密 `RepoKey` 后传输

**参考**: `deve-note plan/05_network.md` E2EE 架构

### C.2 客户端解密

**目标**: 替换 `editor/sync.rs` 中的占位逻辑。

**当前代码** (需替换):
```rust
ServerMessage::SyncPush { ops } => {
    // TODO: Decrypt ops using RepoKey
    for enc_op in ops {
        leptos::logging::warn!("Skipping encrypted op seq: {}", enc_op.seq);
    }
}
```

**替换为**: 使用 `RepoKey` + AES-256-GCM 解密每个 `EncryptedOp`。

**涉及文件**:
- `apps/web/src/editor/sync.rs` — 核心修改点

### C.3 WASM 加密兼容

**目标**: 确保 `cipher.rs` 的 AES-256-GCM 在 `wasm32-unknown-unknown` 下可编译。

**涉及文件**:
- `crates/core/src/security/cipher.rs` — 可能需条件编译 (`aes-gcm` 纯 Rust, 应该可直接用)
- 验证: `cargo check --target=wasm32-unknown-unknown --package deve_core`

**不变量**: `RepoKey` 只在内存中存在, 页面卸载时清除 (不写入 localStorage/IndexedDB)。

---

## Branch D: `feat/plugin-system`

> 进度表 `03_extensions.md` 当前 0%
> 规范来源: `deve-note plan/11_plugins.md`

### D.1 Rhai 运行时完善

**目标**: 补全 Host API。

**涉及文件**:
- `crates/core/src/plugin/runtime/rhai_v1.rs` — 已有骨架, 补全:
  - `host_read_file(path)` — 读取 Vault 文件
  - `host_write_file(path, content)` — 写入 Vault 文件
  - `host_http_get(url)` / `host_http_post(url, body)` — HTTP 请求 (需超时)
  - `host_notify(message)` — 向前端推送通知
- `crates/core/src/plugin/runtime/host/` — Host function 实现

**约束**: Rhai 引擎 MUST 设置 `Engine::set_max_operations()` 和内存上限, 防止恶意脚本。

### D.2 插件清单解析

**目标**: 定义 JSON 格式的插件清单。

**涉及文件**:
- `crates/core/src/plugin/manifest.rs` — 完善:
  ```json
  {
    "name": "ai-chat",
    "version": "0.1.0",
    "engine": "rhai",
    "entry_point": "main.rhai",
    "permissions": ["file:read", "http:get"]
  }
  ```

### D.3 插件加载器

**目标**: 从 `plugins/` 目录自动发现和加载 `.rhai` 脚本。

**涉及文件**:
- `crates/core/src/plugin/loader.rs` — 扫描 `plugins/*/manifest.json`, 加载并注册

### D.4 Chat 面板对接

**目标**: 前端 Chat UI 对接流式输出。

**涉及文件**:
- `crates/core/src/plugin/runtime/chat_stream.rs` — 流式 Token 输出
- `apps/web/src/components/chat/` — 已有 UI 骨架, 对接 WS 流

### D.5 AI Provider 接口

**目标**: 定义标准化 AI 插件 SDK。

**涉及文件**:
- `crates/core/src/plugin/runtime/` — 新增 `provider.rs`:
  - `trait AiProvider: Send + Sync { fn send_message(...) -> Stream<Token> }`
- `plugins/ai-chat/` — 示例实现

**注意**: WASM 运行时 (extism/wasmtime) 本阶段暂缓, 仅做 Rhai。

---

## Branch E: `docs/progress-sync`

> 纯文档更新, 无代码修改

### E.1 更新 `schedules/01_core.md`

Auth 章节 3 项已完成但未打勾:
- `[ ] Argon2` → `[x]` (已实现: `security/auth/password.rs`)
- `[ ] Rate Limiting` → `[x]` (已实现: `server/rate_limit.rs`)
- `[ ] Localhost Policy` → `[x]` (已实现: `AuthConfig.allow_anonymous_localhost`)

### E.2 更新 `deve-note schedule.md`

- 当前写 "Phase 3 完成" → 更新为 "Phase 4 进行中 + Apps Audit 修复完成"
- 更新进度百分比: Core 95% → 98%, UI 90% → 93%

### E.3 更新 `deve-note gaps.md`

- §2.1 Auth Gap → 标记已解决 (JWT + Argon2 + Rate Limiting)
- §1.2 Merge Engine → 与 01_core schedule 状态对齐

### E.4 更新 `deve-note current.md`

- 补充 `server/auth/` 模块文件树描述 (handlers, middleware, brute_force, headers)
- 补充 `security/auth/` 模块重构说明 (password, jwt, config)

---

## 合并顺序

```
main ─┬─ Branch E (docs)        ← 最先合并 (无冲突)
      ├─ Branch C (e2ee)        ← 独立, 随时可合
      ├─ Branch B (dashboard)   ← 独立, 随时可合
      ├─ Branch D (plugins)     ← 独立, 随时可合
      └─ Branch A (css+icons)   ← 最后合并 (改动最广)
```

**B / C / D 之间完全独立, 可同时并行开发。**

### 冲突热点分析

| 文件 | 可能被触及的分支 | 风险 |
|:-----|:--------------|:-----|
| `crates/core/src/protocol.rs` | B (SystemMetrics) + C (KeyExchange) | 低: 都是追加枚举变体 |
| `apps/web/src/app.rs` | A (组件路径) + B (Dashboard 路由) | 低: A 改 import, B 加路由 |
| `apps/web/src/components/chat/**` | A (目录改名) + D (功能实现) | 中: 建议 A 先合并 chat 重构, D 基于其开发 |

### Agent 通用指令

每个分支的 Agent 必须遵守:
1. **文件内聚规则**: 按职责/API/复用边界拆分；超过 250 行需复查，超过 500 行需拆分或说明例外
2. **编译验证**: 每步完成后 `cargo check --package <pkg>` = 0 errors, 0 warnings
3. **测试**: 新增功能必须附带单元测试, 覆盖率 ≥ 80%
4. **路径处理**: 统一使用 `deve_core::utils::path::to_forward_slash` 正斜杠转换
5. **768 MB 约束**: 引入新依赖前评估内存影响
6. **不变量注释**: 复杂逻辑必须标注 Invariants / Pre-conditions / Post-conditions

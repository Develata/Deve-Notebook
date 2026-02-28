# 剩余工作分支规划 (Next Tasks — Branch Decomposition)

> **生成日期**: 2026-02-28
> **前置**: P0 (4/4) ✅ | P1 (7/7) ✅ | P2 (6/6) ✅ | P3 (6/8) 进行中

## 概览

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
- 重构后每个文件 < 130 行
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
1. **文件行数铁律**: 目标 < 130 行, 硬限 250 行
2. **编译验证**: 每步完成后 `cargo check --package <pkg>` = 0 errors, 0 warnings
3. **测试**: 新增功能必须附带单元测试, 覆盖率 ≥ 80%
4. **路径处理**: 统一使用 `deve_core::utils::path::to_forward_slash` 正斜杠转换
5. **768 MB 约束**: 引入新依赖前评估内存影响
6. **不变量注释**: 复杂逻辑必须标注 Invariants / Pre-conditions / Post-conditions

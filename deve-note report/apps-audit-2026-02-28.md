# 📋 Deve-Note `apps/` 全面代码审查报告

**审查日期**: 2026-02-28  
**审查范围**: `apps/cli/` (Rust 后端) + `apps/web/` (Leptos/WASM 前端 + JS 扩展)  
**对照基准**: `deve-note plan/` 全部设计文档 (01-15)  
**审查标准**: AGENTS.md 工程规范 (130 行目标 / 250 行硬限)

---

## 目录

- [第一部分: Plan 与实际代码差距](#第一部分-plan-与实际代码差距)
- [第二部分: Plan 设计不合理之处](#第二部分-plan-设计不合理之处)
- [第三部分: 代码逻辑错误与 Bug](#第三部分-代码逻辑错误与-bug)
- [第四部分: 文件行数违规](#第四部分-文件行数违规)
- [第五部分: 安全问题](#第五部分-安全问题)
- [第六部分: 修复建议优先级排序](#第六部分-修复建议优先级排序)

---

## 第一部分: Plan 与实际代码差距

### 1.1 合规矩阵 (Plan Compliance Matrix)

| # | Plan 设计项 | 出处 | 实现状态 | 严重度 |
|---|:-----------|:-----|:---------|:-------|
| 1 | JWT 认证体系 | `09_auth.md` | ❌ **缺失** | **CRITICAL** |
| 2 | WebSocket 握手鉴权 | `09_auth.md` | ❌ **缺失** (HTTP 层无认证) | **CRITICAL** |
| 3 | 速率限制 (Rate Limiting) | `09_auth.md` | ❌ **缺失** | **HIGH** |
| 4 | CSRF 防护 | `09_auth.md` | ❌ **缺失** | **HIGH** |
| 5 | 服务器仪表盘 (Dashboard) | `08_ui_design_01_web.md` §2 | ❌ **缺失** | **MEDIUM** |
| 6 | E2EE 客户端解密 | `05_network.md` | ⚠️ 服务端实现,客户端为 TODO | **HIGH** |
| 7 | i18n 使用 leptos_i18n | `10_i18n.md` | ⚠️ 自研 match 替代 | **LOW** |
| 8 | CSS Design Token 系统 | `08_ui_design.md` §2.1 | ❌ 全部 Tailwind 硬编码 | **HIGH** |
| 9 | Z-Index 注册表 | `08_ui_design.md` §2.2 | ❌ 值与规范不符 | **MEDIUM** |
| 10 | 图标使用 lucide-leptos | `08_ui_design.md` §2.3 | ❌ 全部内联 SVG | **MEDIUM** |
| 11 | 断连锁定 Overlay | `08_ui_design_01_web.md` §2.3 | ✅ 已实现 | — |
| 12 | 旁观者模式 (Spectator) | `06_repository.md` | ✅ 已实现 | — |
| 13 | Inode 重命名检测 | `04_storage.md` | ✅ 已实现 | — |
| 14 | UUID-First 检索 | `02_positioning.md` | ⚠️ 基本符合 | **LOW** |
| 15 | 清洁文件策略 (Zero Injection) | `04_storage.md` | ✅ 已实现 | — |
| 16 | 组件目录结构 | `08_ui_design.md` §3 | ⚠️ 缺少 `overlay/` `shared/` | **LOW** |

**合规率**: 已实现 4/16 (25%) | 部分实现 4/16 (25%) | 缺失 8/16 (50%)

### 1.2 关键差距详解

#### GAP-1: JWT 认证体系完全缺失 [CRITICAL]

**Plan 要求** (`09_auth.md`):
- 12-Factor Auth, Argon2 密码哈希 + JWT 凭证
- JWT Payload 含 `sub: "admin"`, `exp`
- HttpOnly Cookie 存储
- 环境变量 `AUTH_SECRET`, `AUTH_USER`, `AUTH_PASS`

**实际状态**: 仅存在 P2P Ed25519 握手认证，**无用户登录系统**。

- 无 JWT 库依赖 (未引入 `jsonwebtoken` 等 crate)
- 无 User model、登录/注册接口
- `AUTH_USER` / `AUTH_PASS` 环境变量虽在 plan 中定义，代码中未读取
- WebSocket 入口 `ws/mod.rs` 直接分配随机 UUID，无任何认证:
  ```rust
  let peer_id = uuid::Uuid::new_v4().to_string(); // 零鉴权
  ```

**影响**: 任何客户端都能直接连接 WebSocket 进行读写操作，在公网部署时为严重安全漏洞。

#### GAP-2: WebSocket 握手无鉴权 [CRITICAL]

**Plan 要求** (`09_auth.md`): "WebSocket Auth: 必须在握手阶段验证 Ticket/Token"

**实际状态**: HTTP Upgrade 层无任何 Token/Cookie 校验。P2P 层的 `SyncHello` 签名验证存在但位于协议层，攻击者可跳过直接发送其他消息类型。

#### GAP-3: 速率限制缺失 [HIGH]

**Plan 要求** (`09_auth.md`): "Rate Limiting 必须实施速率限制"

**实际状态**: 全代码库搜索 `rate_limit`, `throttle`, `RateLimit` 均无结果。WebSocket 仅有 16MB 消息大小限制 (DoS 防护不等于速率限制)。

#### GAP-4: CSS Design Token 未实现 [HIGH]

**Plan 要求** (`08_ui_design.md` §2.1): 严格使用 CSS 变量 `--bg-app`, `--fg-primary` 等，**严禁硬编码 Hex 值**。

**实际状态**: 全部组件使用 Tailwind 原子类 (`bg-blue-50`, `text-gray-700`, `bg-white`)。`_variables.css` 中虽定义了部分 CSS 变量 (如 `--bg-app`), 但组件代码几乎不引用这些变量。

**影响**: 主题切换功能无法实现，暗色/亮色模式切换需要重写所有组件样式。

#### GAP-5: E2EE 客户端链路断裂 [HIGH]

**Plan 要求** (`05_network.md`): 全链路 AES-256-GCM 加密

**实际状态**:
- ✅ 服务端 `crates/core/src/security/cipher.rs` — 完整 AES-256-GCM 加解密
- ✅ 服务端 `sync/engine/transfer/` — 发送端加密 + 接收端解密
- ❌ 客户端 `editor/sync.rs` — 加密 Ops 被直接跳过:
  ```rust
  ServerMessage::SyncPush { ops } => {
      // TODO: Decrypt ops using RepoKey
      for enc_op in ops {
          leptos::logging::warn!("Skipping encrypted op seq: {}", enc_op.seq);
      }
  }
  ```
  密钥交换协议尚未实现，客户端无法获取 `RepoKey`。

#### GAP-6: 服务器仪表盘未实现 [MEDIUM]

**Plan 要求** (`08_ui_design_01_web.md` §2): 根路径 `/` 显示 System Health (CPU/RAM/Uptime), Sync Status, Storage Stats, Actions 面板。

**实际状态**: 无 `/api/metrics` 端点、无 CPU/内存采集、无仪表盘 UI 组件。当前根路径直接进入编辑器。

---

## 第二部分: Plan 设计不合理之处

### 2.1 文件行数限制自相矛盾 [MEDIUM]

**问题**: Plan 主文档 (`deve-note plan.md`) 规定 "单文件行数限制: 目标 ~100 行，MUST NOT 超过 200 行"。而 `AGENTS.md` 规定 "目标 < 130 行，硬限 250 行"。两份文档存在冲突。

**建议**: 统一为 AGENTS.md 的 130/250 标准 (更实际)。更新 `deve-note plan.md` 中的表述。

### 2.2 Web 端 "禁用 IndexedDB" 策略过于严格 [MEDIUM]

**问题** (`05_network.md` + `08_ui_design_01_web.md`): Plan 规定 "Web 端严禁使用 IndexedDB/LocalStorage 存储业务数据" 且 "RAM-Only"。但同时 `13_settings.md` 和 `08_ui_design_01_web.md` §5 又要求 "伸缩宽度、配置项等 MUST 通过 localStorage 持久化"。

**矛盾点**: 一方面禁止持久化存储，另一方面要求 UI 布局偏好必须持久化。区分"业务数据"和"UI 偏好数据"在表述上模糊。

**建议**: 明确划分:
- **禁止**: 文档内容、Ledger 数据、同步状态等业务数据
- **允许**: UI 偏好 (侧边栏宽度、主题、语言) 通过 localStorage 存储

### 2.3 "CORS 允许所有" 与 "CSRF 防护" 矛盾 [HIGH]

**问题**: Plan `09_auth.md` 要求 CSRF 防护，但 Server 代码中使用 `CorsLayer::allow_origin(Any)`。通配 CORS 配置天然抵消了 CSRF 防护的意义 — 任何第三方网页都可发起跨域请求。

**建议**: Plan 中应明确 CORS 策略:
- 生产环境: `allow_origin` 限制为 `same-origin` 或明确的域名列表
- 开发环境: 可放宽但需要显著标记

### 2.4 Desktop/Mobile "MUST 原生 UI" 与现实冲突 [LOW]

**问题** (`08_ui_design_02_desktop.md` §4.1, `08_ui_design_03_mobile.md` §7.1): Plan 规定 Desktop/Mobile "MUST 以原生 UI 为标准实现"，但技术栈 (`14_tech_stack.md`) 选定 Tauri v2 (本质上是 WebView + Rust Backend)。Tauri 的前端层仍然是 Web 技术渲染。

**建议**: 将措辞修改为 "MUST 提供原生级体验 (Native-feel)"，或改用 "MUST 使用原生外壳 (Native Shell) + 内嵌 WebView 的混合方案" 以匹配实际技术路线。

### 2.5 single-file 行数限制对 JS bundle 不适用 [LOW]

**问题**: `editor.bundle.js` (2614 行) 是 Webpack/Rollup 打包产物。AGENTS.md 的 130/250 行限制针对源文件，打包产物不应受限。

**建议**: Plan 中明确排除 `*.bundle.js`、`dist/` 目录下的构建产物。在 `.deveignore` 或 AGENTS.md 中列出豁免清单。

### 2.6 Plan 中 "Loro CRDT" 与实际 "Dissimilar Diff" 不一致 [MEDIUM]

**问题**: `03_rendering.md` 提到 "State Layer: 绑定 Loro CRDT 状态 (Ledger)"，`07_diff_logic.md` 提到 "Auto Mode (CRDT): 利用 Loro 的 Op-based Merge"。但 `14_tech_stack.md` 技术栈表中 Diff 引擎标注为 "Dissimilar (Verified)"，Loro CRDT 未出现在技术栈中。

实际代码 `crates/core/` 使用自研的 Op-based 日志 + `dissimilar` crate 计算 diff，**并未引入 Loro CRDT**。

**建议**: 统一 Plan 表述: 要么移除 Loro 引用改为 "自研 Op-based Sync"，要么补充 Loro 的引入计划与时间线。

---

## 第三部分: 代码逻辑错误与 Bug

### 3.1 CRITICAL 级别

#### BUG-C1: `serve.rs` Proxy 模式代码完整重复 (Duplicated Code)

**文件**: `apps/cli/src/commands/serve.rs` L27-L65 与 L73-L110  
**问题**: 端口绑定失败路径与数据库打开失败路径的 proxy 模式初始化代码 **完全一致** (~30 行):
```
RemoteSourceControlApi → host::set_repository → PluginLoader → find_free_port → set_node_role → start_plugin_host_only
```
**风险**: 修改其中一处时极易遗忘另一处，导致行为不一致。  
**修复**: 提取 `start_proxy_mode(main_port: u16) -> Result<()>` 公共函数。

#### BUG-C2: `applyRemoteOpsBatch()` 逐条 dispatch, O(N²) 性能退化

**文件**: `apps/web/js/editor_adapter.js` L205-L225  
**问题**: 每个 Op 都调用一次 `activeView.dispatch()`。对 N 个操作，CodeMirror 执行 N 次 DOM 更新和 N 次扩展重计算。当 Snapshot 带数百个 delta_ops 时严重卡顿。  
**修复**: 收集所有 changes 后一次性 dispatch:
```javascript
const allChanges = ops.map(op => {
  if (op.Insert) return { from: op.Insert.pos, insert: op.Insert.content };
  if (op.Delete) return { from: op.Delete.pos, to: op.Delete.pos + op.Delete.len };
}).filter(Boolean);
activeView.dispatch({ changes: allChanges, annotations: [/* remote annotation */] });
```
注意: 需要使用 `ChangeSet.compose` 处理位置偏移，或按逆序排列操作。

#### BUG-C3: `on_delta.forget()` 每次文档切换内存泄漏

**文件**: `apps/web/src/editor/hook.rs` ~L155  
**问题**: 每次切换文档时 `use_editor` 重新执行，创建新的 `Closure` 并 `.forget()`。旧 Closure 永不回收，持有对 `WsService`、`set_content` 等信号的引用。  
**修复**: 将 Closure 存储在 `StoredValue` 中，在 `on_cleanup` 回调时显式 drop；或将 Closure 生命周期绑定到 Leptos `Owner`。

#### BUG-C4: `block_on` 在 Tokio 异步上下文中导致潜在死锁

**文件**: `apps/cli/src/server/source_control_proxy.rs` (全部 6 个 trait 方法)  
**问题**: 
```rust
tokio::runtime::Handle::current().block_on(async { ... })
```
如果从 tokio 工作线程调用（如 Axum handler 中），`block_on` 阻塞当前线程等待 future 完成，但 future 本身需要工作线程来执行 → **死锁**。  
**修复**: 使用 `tokio::task::block_in_place(|| Handle::current().block_on(...))` 或将 trait 改为 async。同样影响 `mcp/http.rs` 和 `mcp/stdio.rs`。

### 3.2 HIGH 级别

#### BUG-H1: RwLock `.unwrap()` 导致级联 Panic

**影响文件**: 10+ 个 handler 文件  
**典型位置**:
- `handlers/merge.rs`: `state.sync_engine.read().unwrap()`
- `handlers/sync.rs`: `state.sync_engine.write().unwrap()`
- `handlers/listing.rs`: `state.tree_manager.write().unwrap()`
- `handlers/docs/copy.rs`: `state.tree_manager.write().unwrap()`

**问题**: 若任何线程持有 RwLock 时 panic, 锁将 **中毒 (poisoned)**。此后所有 `.unwrap()` 调用都会 panic，造成级联崩溃。  
**修复**: 改为 `.read().unwrap_or_else(|e| e.into_inner())` 或返回 `500 Internal Error`。

#### BUG-H2: `ffi.rs` — `to_op()` Replace 场景丢失 Insert 数据

**文件**: `apps/web/src/editor/ffi.rs` L80-L88  
**问题**:
```rust
(true, true) => {
    // Replace = Delete + Insert. For simplicity, return only the more significant one.
    Some(Op::Delete { pos, len })
}
```
Replace 操作只返回 Delete，Insert 内容被静默丢弃。虽标记 `#[allow(dead_code)]` 且实际使用 `to_ops()`，但这是一个潜伏的逻辑炸弹。  
**修复**: 删除 `to_op()` 方法或修正为返回 `Vec<Op>`。确保只暴露 `to_ops()` 作为公共接口。

#### BUG-H3: VisualViewport 事件监听器永不移除 (移动端内存泄漏)

**文件**: `apps/web/src/components/mobile_layout/effects.rs` ~L84-94  
**问题**: `resize_cb.forget()` + `scroll_cb.forget()` 泄漏 Closure 且从未调用 `removeEventListener`。移动端布局反复挂载/卸载时累积孤立监听器。  
**修复**: 在 `on_cleanup` 中主动移除监听器并 drop Closure。

#### BUG-H4: `watch.rs` 竞态条件

**文件**: `apps/cli/src/commands/watch.rs` L33-42  
**问题**: 
```rust
watcher.watch()?;           // ← 先启动 (可能阻塞)
ctrlc::set_handler(...)?;   // ← 后注册 Ctrl+C
```
若 `watcher.watch()` 阻塞，`ctrlc::set_handler` 永不执行。即使不阻塞，两者之间存在时间窗口。  
**修复**: 先注册 Ctrl+C handler，再启动 watcher。

#### BUG-H5: `FileReader onload` Closure 泄漏

**文件**: `apps/web/src/components/chat/drop_handler.rs` ~L57  
**问题**: `onload.forget()` 导致每次文件拖拽上传都泄漏一个 Closure。  
**修复**: 将 Closure 存储为临时变量，在读取完成后清理。

#### BUG-H6: `handle_server_message` 14 个参数 — "上帝函数"

**文件**: `apps/web/src/editor/sync.rs` L18-33  
**问题**: 函数接受 14 个参数，`#[allow(clippy::too_many_arguments)]` 压制 Clippy。职责混杂 (snapshot 处理 + diff 应用 + playback + 进度条 + 统计)。  
**修复**: 封装为 `SyncContext` 结构体:
```rust
pub struct SyncContext {
    doc_id: ReadSignal<Option<DocId>>,
    ws: WsService,
    content: WriteSignal<String>,
    // ... grouped by concern
}
```

### 3.3 MEDIUM 级别

#### BUG-M1: `commands/init.rs` — `_path` 参数被忽略

**问题**: CLI `--path` 参数完全未使用 (变量名 `_path`)。用户执行 `deve init --path /tmp/test` 时误以为在指定目录初始化。

#### BUG-M2: `node_role.rs` — `OnceLock` 只能设置一次

**问题**: `set_node_role()` 使用 `OnceLock::set()`, 第二次调用静默失败。若需要从 main 切换到 proxy 模式，角色无法更新。  
**修复**: 使用 `ArcSwap` 或 `tokio::sync::watch` 替代。

#### BUG-M3: `prewarm.rs` — 静默吞没快照保存错误

**问题**: `let _ = repo.save_snapshot(...)` — 快照保存失败时无日志。  
**修复**: 至少记录 `tracing::warn!`。

#### BUG-M4: 协议不对称 — 发送 JSON / 接收 Bincode

**文件**: `apps/web/src/api/output.rs` vs `connection.rs`  
**问题**: 接收端优先 Bincode 降级 JSON, 但发送端仅使用 JSON。若服务端未来期望 Bincode, 将导致解析错误。  
**修复**: 发送端应同样支持 Bincode 编码以匹配接收策略。

---

## 第四部分: 文件行数违规

### 4.1 超过 250 行硬限制 (MUST 立即重构)

| 文件 | 行数 | 拆分建议 |
|:-----|:-----|:---------|
| `apps/web/src/components/search_box/file_ops.rs` | **380** | → `parser.rs` + `normalize.rs` + `candidates.rs` |
| `apps/web/js/editor_adapter.js` | **288** | → `init.js` + `remote_ops.js` + `api.js` |
| `apps/web/src/editor/sync.rs` | **264** | → 按消息类型拆分: `sync_snapshot.rs` + `sync_delta.rs` + `sync_push.rs` |
| `apps/cli/src/server/mod.rs` | **248** | → `build_router.rs` + `build_state.rs` + `spawn_watcher.rs` |

### 4.2 超过 130 行目标限制 (SHOULD 尽快重构)

**CLI 后端** (共 13 个文件超标):

| 文件 | 行数 |
|:-----|:-----|
| `server/handlers/sync.rs` | 213 |
| `server/handlers/document.rs` | 213 |
| `server/handlers/docs/copy.rs` | 200 |
| `server/handlers/switcher.rs` | 186 |
| `server/ai_chat/sse_parser.rs` | 182 |
| `server/agent_bridge.rs` | 167 |
| `commands/serve.rs` | 163 |
| `server/handlers/source_control/staging.rs` | 158 |
| `server/handlers/docs/create.rs` | 156 |
| `server/handlers/merge.rs` | 148 |
| `server/handlers/docs/copy_utils.rs` | 142 |
| `server/handlers/source_control/diff.rs` | 135 |

**Web 前端** (共 25+ 个文件超标):

| 文件 | 行数 |
|:-----|:-----|
| `js/extensions/hybrid.js` | 234 |
| `components/mobile_layout/footer.rs` | 231 |
| `hooks/use_core/effects.rs` | 228 |
| `components/search_box/providers.rs` | 219 |
| `hooks/use_core/callbacks.rs` | 218 |
| `components/mobile_layout/drawers/left.rs` | 215 |
| `components/mobile_layout/mod.rs` | 214 |
| `editor/hook.rs` | 191 |
| `components/search_box/result_item.rs` | 190 |
| `components/main_layout.rs` | 187 |
| `hooks/use_core/state.rs` | 187 |
| `api/connection.rs` | 186 |
| `js/extensions/mermaid.js` | 186 |
| `components/sidebar/item.rs` | 185 |
| `components/command_palette/mod.rs` | 185 |
| `hooks/use_core/apply.rs` | 182 |
| `components/activity_bar.rs` | 177 |
| `components/desktop_layout.rs` | 176 |
| `hooks/use_core/mod.rs` | 175 |
| `js/extensions/math_parser.js` | 166 |
| `api/output.rs` | 158 |
| `api/mod.rs` | 157 |
| `js/extensions/table_parser.js` | 141 |
| `js/extensions/math.js` | 139 |
| `js/extensions/hyperlink_click.js` | 139 |
| `editor/mod.rs` | 137 |

---

## 第五部分: 安全问题

### 5.1 CRITICAL

#### SEC-C1: CORS 允许所有来源

**文件**: `apps/cli/src/server/mod.rs` L217-220
```rust
CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any)
```
任意第三方网页可向服务器发起跨域请求，配合无 JWT 认证，可直接读写用户数据。  
**修复**: 限制 `allow_origin` 为 `http://localhost:{port}` 或具体域名。

#### SEC-C2: Mermaid XSS 注入

**文件**: `apps/web/js/extensions/mermaid.js` L11
```javascript
securityLevel: 'loose', // 允许 HTML 标签
```
`'loose'` 允许在 Mermaid 图表中嵌入任意 HTML，攻击者可注入 `<img onerror=alert(1)>` 实现 XSS。  
**修复**: 改为 `'strict'` 或 `'sandbox'` (一行修复)。

### 5.2 HIGH

#### SEC-H1: WebSocket 使用明文 `ws://`

**文件**: `apps/web/src/api/connection.rs` ~L159
```rust
format!("ws://{}:{}/ws", hostname, port)
```
公网部署时所有编辑内容和认证信息明文传输。  
**修复**: 根据页面协议自动选择:
```rust
let protocol = if is_https() { "wss" } else { "ws" };
```

#### SEC-H2: 密钥文件权限未设置

**文件**: `apps/cli/src/server/security.rs`  
`identity.key` 和 `repo.key` 使用 `std::fs::write()` 写入，默认权限 0644 (world-readable)。  
**修复**: Unix 上使用 `std::os::unix::fs::PermissionsExt` 设为 `0600`。

#### SEC-H3: `expect()` 在 WASM 环境导致不可恢复 Panic

**文件**: `apps/web/src/api/connection.rs` ~L157
```rust
.expect("Window 对象不存在 (非浏览器环境?)")
```
WASM 中 panic 抛出不可恢复的 JS 异常，整个应用崩溃。  
**修复**: 改为 `.ok().and_then(|w| w.location().hostname().ok()).unwrap_or_else(|| "localhost".to_string())`。

### 5.3 MEDIUM

#### SEC-M1: 生产代码残留 `console.log`

| 文件 | 内容 |
|:-----|:-----|
| `js/extensions/hybrid.js` | `console.log("[HybridDebug] QuoteMark found at:", ...)` |
| `js/extensions/checkbox_ext.js` | `console.log("Loading Checkbox Extension...")` |
| `js/extensions/table.js` | `console.log("Header Data:", ...)` |
| `js/editor_adapter.js` | `console.log("Editor Adapter v5 - ...")` |

可能泄漏内部实现细节。应全部移除或替换为条件编译的 debug 日志。

---

## 第六部分: 修复建议优先级排序

### P0 — 部署阻塞 (Deploy Blockers)

| # | 问题 | 修复成本 | 位置 |
|---|:-----|:---------|:-----|
| 1 | SEC-C2: Mermaid `securityLevel: 'loose'` | 1 行 | `mermaid.js` |
| 2 | SEC-C1: CORS `allow_origin(Any)` | 5 行 | `server/mod.rs` |
| 3 | BUG-C4: `block_on` 潜在死锁 | 20 行 | `source_control_proxy.rs` |
| 4 | SEC-H3: WASM `expect()` panic | 3 行 | `api/connection.rs` |

### P1 — 高优先级 (1-2 周内修复)

| # | 问题 | 修复成本 | 位置 |
|---|:-----|:---------|:-----|
| 5 | GAP-1: JWT 认证体系 | 大型功能 | 新增 `auth/` 模块 |
| 6 | GAP-3: 速率限制 | 中型功能 | `server/mod.rs` 路由层 |
| 7 | BUG-C2: `applyRemoteOpsBatch` O(N²) | 30 行 | `editor_adapter.js` |
| 8 | BUG-C3: `on_delta.forget()` 内存泄漏 | 15 行 | `editor/hook.rs` |
| 9 | SEC-H1: `ws://` → `wss://` 自适应 | 10 行 | `api/connection.rs` |
| 10 | SEC-H2: 密钥文件权限 | 10 行 | `server/security.rs` |
| 11 | BUG-H1: RwLock `.unwrap()` 级联 panic | 30 行 | 10+ handler 文件 |

### P2 — 中优先级 (1 个月内修复)

| # | 问题 | 修复成本 | 位置 |
|---|:-----|:---------|:-----|
| 12 | 行数违规: 4 个文件超硬限 | 各拆为 3 个文件 | 见 §4.1 |
| 13 | BUG-C1: `serve.rs` proxy 代码重复 | 15 行 | `commands/serve.rs` |
| 14 | BUG-H3: VisualViewport 内存泄漏 | 20 行 | `mobile_layout/effects.rs` |
| 15 | GAP-4: CSS Design Token 迁移 | 大型工作 | 全部组件 |
| 16 | i18n 硬编码修复 | 中型工作 | ~12 处位置 |
| 17 | console.log 清理 | 小型工作 | 4 个 JS 文件 |

### P3 — 低优先级 (按需修复)

| # | 问题 | 位置 |
|---|:-----|:-----|
| 18 | GAP-6: 服务器仪表盘 | 新增功能 |
| 19 | GAP-5: E2EE 客户端密钥交换 | `editor/sync.rs` |
| 20 | 组件目录结构规范化 | `components/` |
| 21 | lucide-leptos 图标迁移 | 全部组件 |
| 22 | CoreState 拆分为独立 Context | `hooks/use_core/` |
| 23 | Plan 文档矛盾修正 | `deve-note plan/` |

---

## 附录: 合规项 (已正确实现)

以下设计点已正确实现，表现良好:

| 功能 | 验证结果 |
|:-----|:---------|
| 断连锁定 Overlay | ✅ 全屏遮罩 + 重连提示 |
| 旁观者模式 (Spectator Mode) | ✅ 编辑器只读 + 水印 + 状态栏 |
| Inode 重命名检测 | ✅ `file_id` crate 跨平台支持 |
| 清洁文件策略 (Zero Injection) | ✅ 无 UUID 注入到 Markdown |
| 指数退避重连 | ✅ 1s-10s BackoffStrategy |
| 心跳保活 (Ping) | ✅ 30s 间隔 |
| 离线消息队列 | ✅ 500 条上限 |
| 外部链接安全 | ✅ `noopener, noreferrer` |
| Watcher 防抖 | ✅ Debouncer 机制 |
| P2P Ed25519 签名验证 | ✅ 握手层已实现 |

---

**审查结论**: 项目核心架构 (Ledger/Vault 三库隔离、Op-based 同步、文件树管理) 实现扎实。主要薄弱环节集中在 **安全层** (认证/鉴权) 和 **前端工程规范** (行数限制/Design Token/i18n)。建议优先处理 P0 级安全修复 (4 项，预计 1 天)，随后推进 JWT 认证体系的实现。

# Plan 设计文档审查报告 — 修改建议

**日期**: 2026-02-28
**范围**: `deve-note plan/` 全部 15+ 文档
**目标**: 发现并修复所有内部矛盾、规范缺失、过度承诺与设计歧义

---

## 目录

1. [P0 — 内部矛盾 (Critical Contradictions)](#1-p0--内部矛盾)
2. [P1 — 规范缺失 (Missing Specifications)](#2-p1--规范缺失)
3. [P2 — 设计质量 (Design Quality)](#3-p2--设计质量)
4. [P3 — 文档结构 (Document Hygiene)](#4-p3--文档结构)
5. [逐文件修改清单 (File-by-File Fix List)](#5-逐文件修改清单)

---

## 1. P0 — 内部矛盾

### 1.1 文件行数上限不一致

| 来源 | Target | Hard Limit |
|:---|:---|:---|
| `deve-note plan.md` L42 | ~100 行 | 200 行 |
| `AGENTS.md` §2 | < 130 行 | 250 行 |
| `08_ui_design_03_mobile.md` §8.3 | < 130 行 | 250 行 |

**分析**: 三处定义两套标准。AGENTS.md 的 130/250 方案是最新且被实际执行的标准（代码审计中以此为依据）。

**修改建议**: 统一采用 AGENTS.md 标准。
- `deve-note plan.md` L42 改为:
  ```
  *   **单文件行数限制**: 目标 < 130 行，MUST NOT 超过 250 行 (熔断阈值)。
  ```

---

### 1.2 Loro CRDT vs Dissimilar 矛盾

| 来源 | 表述 |
|:---|:---|
| `03_rendering.md` L4 | "绑定 Loro CRDT 状态 (Ledger)" |
| `07_diff_logic.md` 和解策略 | "利用 Loro 的 Op-based Merge" |
| `14_tech_stack.md` Diff 行 | **Dissimilar** (Verified) |
| `Cargo.toml` L37 | `# loro = "1.0" # TBD` (注释掉) |
| 实际依赖 | `dissimilar 1.0.10` + `similar 2.7.0` |

**分析**: Loro 从未被集成，当前使用自研 Op Log + dissimilar/similar 进行 diff。Plan 中大量 Loro 引用已经过时。

**修改建议**:
- `03_rendering.md` L4: 将 "Loro CRDT 状态 (Ledger)" 改为 "自研 Op-based 状态 (Ledger)"
- `07_diff_logic.md` 和解策略:
  - "利用 Loro 的 Op-based Merge" → "利用自研 Op Log 的 Operation-based Merge"
  - 添加说明: "**技术选型**: 文本 Diff 使用 `dissimilar` (Myers) + `similar` crate，不依赖外部 CRDT 框架。"
- `14_tech_stack.md`: 在 Diff 行下加注释: "**Note**: Loro CRDT 为远期预研方向 (TBD)，当前版本不依赖。"

---

### 1.3 Web 端存储策略矛盾

| 来源 | 表述 |
|:---|:---|
| `05_network.md` Web Client | "严禁使用 IndexedDB/**LocalStorage** 存储业务数据" |
| `08_ui_design.md` §4.2 | "前端配置存储在 **LocalStorage** 的 `deve_config` 键中" |
| `08_ui_design_01_web.md` §5 | "伸缩宽度 MUST 通过 **localStorage** 持久化" |

**分析**: `05_network.md` 的禁令过于宽泛，未区分"业务数据"与"UI 偏好"。`localStorage` 存放纯 UI 配置（侧边栏宽度、主题偏好）是合理的，不违反"Stateless Dashboard"的本意。

**修改建议**: `05_network.md` Web Client 存储条改为:
```markdown
* **存储**：**Stateless / RAM-Only (纯内存)**。Web 端严禁使用 IndexedDB/LocalStorage 存储**业务数据**
  （文档内容、操作日志、同步状态等）。
  **例外**: 纯 UI 偏好（布局宽度、主题选择等）**MAY** 使用 `localStorage`，
  因其不影响 Ledger 真源且断连后无害。
```

---

### 1.4 原生 UI vs Tauri WebView

| 来源 | 表述 |
|:---|:---|
| `08_ui_design_02_desktop.md` §4.1 | "Desktop **MUST** 采用原生 UI 实现" |
| `08_ui_design_03_mobile.md` §7.1 | "Mobile **MUST** 使用原生 UI 实现（非 WebView）" |
| `14_tech_stack.md` Build 行 | **Tauri v2** — "跨平台外壳 (Mobile/Desktop)" |

**分析**: Tauri v2 的核心就是 WebView（WKWebView/WebView2），与"MUST 原生 UI（非 WebView）"直接矛盾。以一人之力用 Swift/Kotlin 写原生 UI 不现实。

**修改建议**:
- `08_ui_design_02_desktop.md` §4.1 改为:
  ```markdown
  ### 4.1 跨平台 UI 方案
  *   **Rule**: Desktop 采用 **Tauri v2 (WebView)** 作为跨平台外壳，前端代码与 Web 端共享。
  *   **Consistency**: 交互与布局规则 MUST 与本章一致。
  *   **Note**: "原生 UI" 在此指用户体验层面（窗口管理、菜单栏、系统托盘等），而非技术实现层面。
  ```
- `08_ui_design_03_mobile.md` §7.1 改为:
  ```markdown
  ### 7.1 移动端 UI 方案
  *   **Rule**: Mobile 采用 **Tauri v2 Mobile** 作为外壳（WKWebView/Android WebView），
        前端代码与 Web 端共享，配合原生层访问摄像头/文件系统/推送等系统 API。
  *   **Consistency**: 交互与布局规则 MUST 与本章一致。
  ```

---

### 1.5 Docker 默认端口 vs CLI 默认端口

| 来源 | 端口 |
|:---|:---|
| `15_release.md` Docker 示例 | `-p 3000:3000` |
| `apps/cli/src/main.rs` L47 | `default_value_t = 3001` |
| `05_network.md` Main/Proxy | "主端口 (默认 3001)" |

**修改建议**: `15_release.md` Docker 示例全部改为 `3001`:
```bash
docker run -d \
  --name deve-server \
  -p 3001:3001 \
  ...
```
Docker Compose 同步修改端口为 `3001:3001`。

---

### 1.6 数据迁移策略矛盾

| 来源 | 表述 |
|:---|:---|
| `04_storage.md` Schema 稳定性 | "SHOULD NOT 变更; 采用 Copy & Rebuild 策略" |
| `15_release.md` §3 Versioning | "MUST 提供数据迁移脚本 (Migration)" |

**分析**: 两者意图不同但表述冲突。`04_storage.md` 的"Copy & Rebuild"是一种迁移策略，只是不用复杂脚本。

**修改建议**: `15_release.md` §3 改为:
```markdown
> **Data Compatibility**: 任何涉及 Ledger/Vault 存储结构的变更 MUST 提供迁移路径。
> 首选 "Copy & Rebuild" 策略（见 04_storage.md）；仅当无法重建时才提供增量迁移脚本。
```

---

## 2. P1 — 规范缺失

### 2.1 Auth 实现细节严重不足 (`09_auth.md`)

**现状**: 仅列出 JWT / Argon2 / CSRF / Rate Limiting 关键词，但无任何实现规格。

**需要补充的内容**:

```markdown
## JWT 规范 (JWT Specification)

*   **Algorithm**: `HS256` (using `AUTH_SECRET` as key).
*   **Payload**:
    ```json
    {
      "sub": "admin",
      "iat": 1700000000,
      "exp": 1700086400
    }
    ```
*   **Lifetime**: Access Token 有效期 `24h`; Refresh Token `7d` (Optional).
*   **Delivery**: `Set-Cookie: token=<jwt>; HttpOnly; Secure; SameSite=Strict; Path=/`.
*   **Refresh**: 客户端检测到 `401` 后请求 `POST /api/auth/refresh`.

## Anti-CSRF 策略

*   **Method**: `SameSite=Strict` Cookie + 双重提交 (Double Submit) 仅适用于非 GET 请求。
*   **Implementation**: 后端中间件在 Set-Cookie 时附加 `csrf_token`，
    前端在请求头 `X-CSRF-Token` 中回传，后端比对。

## Rate Limiting 规范

*   **Login Endpoint** (`POST /api/auth/login`): 5 次/分钟/IP。
*   **API Endpoints**: 60 次/分钟/IP (Authenticated).
*   **WebSocket**: 100 条消息/分钟/连接。
*   **Implementation**: Tower `RateLimitLayer` 或 `governor` crate。

## CORS 策略

*   **Server 模式**: Origin 限制为 `http://localhost:{port}` 和配置的域名白名单。
*   **MUST NOT** 使用 `allow_origin(Any)` (当前代码违反此规则)。

## API Endpoints

| Method | Path | Description |
|:---|:---|:---|
| `POST` | `/api/auth/login` | 用户登录，返回 JWT Cookie |
| `POST` | `/api/auth/logout` | 清除 Cookie |
| `GET` | `/api/auth/me` | 返回当前用户信息 |
```

---

### 2.2 HTTPS/TLS 配置缺失

`09_auth.md` 仅说 "Public Network 必须强制 HTTPS" 但无任何实现指导。

**需要在 `09_auth.md` 或 `13_settings.md` 补充**:

```markdown
## TLS 配置

*   **推荐方案**: 反向代理 (Nginx/Caddy) 终止 TLS，内部 `deve serve` 仅 HTTP。
*   **直连方案 (可选)**: 支持 `--tls-cert` / `--tls-key` 参数直接启用 HTTPS。
*   **Configuration**:
    | Key | Default | Description |
    |:---|:---|:---|
    | `TLS_CERT_PATH` | *(none)* | PEM 证书路径 |
    | `TLS_KEY_PATH` | *(none)* | PEM 私钥路径 |
*   **WebSocket**: 当 TLS 启用时，WS 自动升级为 `wss://`。
```

---

### 2.3 WebSocket 重连协议缺失

`05_network.md` 说"断连即锁屏"但未规范重连策略。

**建议在 `05_network.md` "连接与协议" 下添加**:

```markdown
### WebSocket Reconnection (重连策略)

*   **Strategy**: Exponential Backoff with Jitter。
*   **Intervals**: 1s → 2s → 4s → 8s → 16s → 30s (cap)。
*   **Max Retries**: 无限 (用户手动关闭才停止)。
*   **UI Feedback**:
    *   断连后立即显示 "Reconnecting..." 遮罩。
    *   每次重连尝试更新计数器 "Retry #N..."。
    *   重连成功后自动请求增量同步 (SyncHello)。
*   **State Recovery**: 重连成功后 MUST 发送 `SyncHello` 获取离线期间的变更。
```

---

### 2.4 错误码目录缺失

`10_i18n.md` 规定 "MUST 返回标准错误码" 但全 plan 无任何错误码定义。

**建议在 `10_i18n.md` 末尾补充初始目录**:

```markdown
## Error Code Catalog (错误码目录)

### Authentication Errors (`AUTH_*`)
| Code | HTTP Status | Description |
|:---|:---|:---|
| `AUTH_INVALID_PASSWORD` | 401 | 密码错误 |
| `AUTH_TOKEN_EXPIRED` | 401 | Token 已过期 |
| `AUTH_RATE_LIMITED` | 429 | 请求频率超限 |
| `AUTH_CSRF_MISMATCH` | 403 | CSRF Token 不匹配 |

### Storage Errors (`STORAGE_*`)
| Code | HTTP Status | Description |
|:---|:---|:---|
| `STORAGE_DB_LOCKED` | 503 | 数据库被其他进程锁定 |
| `STORAGE_NOT_FOUND` | 404 | 文档/目录不存在 |
| `STORAGE_CONFLICT` | 409 | 并发写入冲突 |

### Sync Errors (`SYNC_*`)
| Code | HTTP Status | Description |
|:---|:---|:---|
| `SYNC_PEER_UNKNOWN` | 403 | 未知 Peer（未握手） |
| `SYNC_VERSION_MISMATCH` | 409 | 协议版本不兼容 |
| `SYNC_DECRYPT_FAILED` | 400 | 数据解密失败 |
```

---

### 2.5 Dashboard 实现规格不足 (`08_ui_design_01_web.md`)

Dashboard (§2.1) 定义了 Metrics 表格和 `SystemMetrics` 结构体，但缺少:
- API 端点定义 (`GET /api/metrics` 或通过 WS 推送？)
- 权限要求（需要登录才能查看？）
- 前端路由（是 `/` 根路径还是 `/dashboard`？）

**建议补充**:

```markdown
### 2.4 Dashboard 路由与权限
*   **Route**: `/` (根路径，无 DocId 参数时)。
*   **Auth**: Dashboard MUST 要求已认证身份。未认证访问跳转 Login。
*   **Data Channel**: 通过现有 WebSocket 连接推送 `ServerMessage::SystemMetrics`。
*   **Fallback**: WebSocket 断开时，Metrics 冻结并显示 "Disconnected" 状态。
```

---

### 2.6 WASM 堆内存管理缺失

Plan 选择了 Leptos + WASM 前端，但对浏览器 WASM 堆没有任何讨论。大文档场景下易触发 OOM。

**建议在 `14_tech_stack.md` 补充**:

```markdown
## WASM 内存约束

*   **Budget**: 前端 WASM 堆目标 < 64MB (Mobile), < 128MB (Desktop)。
*   **Large Doc Strategy**: 超过 100KB 的文档使用分段加载，不将全文存入 WASM 堆。
*   **Monitoring**: 通过 `wasm_bindgen::memory()` 跟踪实际用量并在 DevTools 输出。
```

---

### 2.7 Backup (备份) 策略未定义

`04_storage.md` 两处写 "Virtual Backup: TBD"，`06_repository.md` 提及 "Virtual Backup Branch" 但无细节。

**建议在 `04_storage.md` 将 TBD 替换为初始方案**:

```markdown
## Virtual Backup (虚拟备份)

*   **Mechanism**: 系统 MAY 为当前活跃 Repo 自动创建 `.redb` 文件的 Copy-on-Write 快照。
*   **Frequency**: 每日自动 (可配) 或手动触发 (`deve backup` 命令)。
*   **Storage**: `ledger/backups/<repo_name>-<timestamp>.redb`。
*   **Retention**: 默认保留最近 3 份；超出按 FIFO 删除。
*   **Note**: 此为 MAY（可选）功能，首次发布不强制实现。
```

---

## 3. P2 — 设计质量

### 3.1 性能剖面映射不清晰

**问题**: `AGENTS.md` 目标 768MB-1GB，`14_tech_stack.md` 定义 Low-Spec 为 512MB、Standard 为 1GB+，`13_settings.md` 有 `DEVE_PROFILE` 配置但无功能映射。

**建议在 `14_tech_stack.md` 补充明确的功能矩阵**:

```markdown
### Profile → Feature Matrix

| Feature | `low-spec` (≤768MB) | `standard` (≥1GB) |
|:---|:---|:---|
| CSR | ✅ | ✅ |
| SSR | ❌ | ✅ |
| Full-Text Search (Tantivy) | ❌ | ✅ |
| Graph Visualization | ❌ | ✅ |
| Snapshot Depth | 10 | 100 |
| MEM_CACHE_MB default | 32 | 128 |
| Plugin Podman | ❌ | ✅ |
```

并将 Low-Spec 从 512MB 改为 768MB 对齐 AGENTS.md。

---

### 3.2 Search Gate 重复定义

"Search Gate"（预加载前禁用搜索）在三处重复:
- `03_rendering.md` 大文档策略
- `05_network.md` OpenDoc 性能策略
- `07_diff_logic.md` 长文档打开策略

**建议**: 在 `03_rendering.md` 保持主定义，其余两处改为引用:
```markdown
*   **Search Gate**: 见 [03_rendering.md §大文档渲染策略](./03_rendering.md)。
```

---

### 3.3 JS Bundle 文件行数限制豁免

AGENTS.md 的 130/250 行限制面向 Rust 源码，但 `apps/web/js/` 下的 JS 文件（如 editor_adapter.js 288 行）本质上是 CodeMirror 桥接层，逻辑不可拆分。

**建议在 AGENTS.md 或 `deve-note plan.md` 添加豁免条款**:

```markdown
*   **例外**: `apps/web/js/` 下的 JavaScript Bridge 文件因 FFI 性质，
    行数限制放宽至 **target < 200 行, hard limit 400 行**。
    超限时应优先提取独立模块 (e.g., `extensions/*.js`)。
```

---

### 3.4 09_auth.md 安全策略节为空

`09_auth.md` 有 "## 安全策略 (Security Policies)" 标题但内容为空。

**建议填充**:

```markdown
## 安全策略 (Security Policies)

*   **Brute Force Protection**: 连续 5 次登录失败后 IP 封禁 15 分钟。
*   **Token Revocation**: 密码修改后所有已签发 JWT 立即失效 (通过版本号机制)。
*   **Headers**: 所有 HTTP 响应 MUST 包含:
    *   `X-Content-Type-Options: nosniff`
    *   `X-Frame-Options: DENY`
    *   `Content-Security-Policy: default-src 'self'`
*   **Key File Permissions**: `identity.key` 文件权限 MUST 设为 `0600` (Owner-only)。
*   **Audit Log**: 登录成功/失败事件 MUST 记录到结构化日志 (Tracing)。
```

---

## 4. P3 — 文档结构

### 4.1 `05_network.md` 重复内容

"Indirect Sync, Case 2" 部分有四行内容被完整复制了两次:
```markdown
    2.  B 检查本地 Trusted List，发现 **不认识 A** (未握手)。
    3.  B **MUST Ignore** A's offer (Strict Filtering).
    4.  C **MUST NOT** 传输 A 的数据给 B (Payload Blocking).
```
**修改**: 删除重复的 3 行。

---

### 4.2 `08_ui_design_03_mobile.md` 节号错乱

§7 "实现策略" 下的子节编号使用了 "4.1"、"4.2"、"4.2.1" 等，与父节 §7 不匹配。应该是从早期版本复制过来时未更新。

**修改**: 重编号为 7.1、7.2、7.2.1、7.2.2、7.2.3、7.3、7.3.1-7.3.5、7.4。

---

### 4.3 `deve-note plan.md` 缺少 Acceptance Cases 索引

Master Index 列出了 01-15 子文档，但未提及 `acceptance-cases/` 目录。

**建议添加**:
```markdown
### Appendix: Acceptance Test Cases
*   **[Acceptance Cases Index](./acceptance-cases/00_index.md)**: 验收用例集。
```

---

### 4.4 部分计划文档超过自身规定的行数限制

| 文件 | 行数 | 超限 |
|:---|:---|:---|
| `08_ui_design_03_mobile.md` | 274 行 | 是 (>250) |
| `05_network.md` | ~170 行 | 否 |

**注**: Plan 文档本身是 Markdown 设计文档，不是代码文件。行数限制严格来说不适用。但如果要保持一致性，`08_ui_design_03_mobile.md` 的 §8 "落地记录" 可以拆分为独立文件 `08_ui_design_03_mobile_changelog.md`。

---

## 5. 逐文件修改清单

### `deve-note plan.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | L42 行数限制 | `~100行/200行` → `< 130行/250行` |
| 2 | 末尾 | 添加 Acceptance Cases 索引 |

### `03_rendering.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | L4 State Layer | "Loro CRDT 状态" → "自研 Op-based 状态" |

### `05_network.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | Web Client 存储 | 添加 localStorage UI 偏好豁免条款 |
| 2 | Case 2 重复行 | 删除 3 行重复内容 |
| 3 | 连接协议后 | 新增 "WebSocket Reconnection" 小节 |

### `07_diff_logic.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | 和解策略 Auto Mode | "Loro" → "自研 Op Log" |
| 2 | Search Gate | 改为交叉引用 03_rendering.md |

### `08_ui_design_02_desktop.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | §4.1 | "原生 UI" → "Tauri v2 WebView" |

### `08_ui_design_03_mobile.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | §7 子节号 | 4.x → 7.x 重编号 |
| 2 | §7.1 | "原生 UI（非 WebView）" → "Tauri v2 Mobile" |
| 3 | §8+ | 考虑拆分落地记录为独立文件 |

### `09_auth.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | JWT 下方 | 新增 JWT 规范 (Algorithm, Payload, Lifetime, Delivery) |
| 2 | 安全下方 | 新增 Anti-CSRF 策略 |
| 3 | 安全下方 | 新增 Rate Limiting 规范 |
| 4 | 安全下方 | 新增 CORS 策略 |
| 5 | 安全下方 | 新增 API Endpoints 表格 |
| 6 | 安全策略 (空节) | 填充安全策略内容 |
| 7 | / | 新增 TLS 配置节 (或移至 13_settings.md) |

### `10_i18n.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | 末尾 | 新增 Error Code Catalog |

### `08_ui_design_01_web.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | §2 Dashboard | 补充路由、权限、数据通道规范 |

### `14_tech_stack.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | Diff 行 | 添加 Loro TBD 说明 |
| 2 | 性能预算 | Low-Spec 改为 768MB；添加 Profile→Feature 矩阵 |
| 3 | 末尾 | 新增 WASM 内存约束节 |

### `15_release.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | Docker 示例 | 端口 `3000` → `3001` |
| 2 | §3 Versioning | 数据迁移措辞与 04_storage.md 对齐 |

### `04_storage.md`
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | Virtual Backup (TBD) | 替换为初始方案 |

### `AGENTS.md` (或 `deve-note plan.md`)
| # | 位置 | 修改 |
|:---|:---|:---|
| 1 | File Size Limits | 添加 JS Bridge 文件豁免条款 |

---

## 附录: 修改优先级排序

| 优先级 | 修改项 | 影响范围 |
|:---|:---|:---|
| **P0-CRITICAL** | 1.1 行数限制统一 | 全项目编码标准 |
| **P0-CRITICAL** | 1.2 Loro→Dissimilar | 引擎选型理解 |
| **P0-CRITICAL** | 1.4 原生UI→Tauri | 构建策略决策 |
| **P0-HIGH** | 1.3 localStorage 豁免 | 前端实现合规性 |
| **P0-HIGH** | 1.5 Docker 端口统一 | 部署文档准确性 |
| **P1-HIGH** | 2.1 Auth 补充 | 安全层实现 |
| **P1-HIGH** | 2.4 错误码目录 | i18n 落地 |
| **P1-MEDIUM** | 2.2 TLS 配置 | 生产部署 |
| **P1-MEDIUM** | 2.3 WS 重连 | 用户体验 |
| **P2-LOW** | 3.x 设计质量 | 技术债务 |
| **P3-LOW** | 4.x 文档结构 | 可维护性 |

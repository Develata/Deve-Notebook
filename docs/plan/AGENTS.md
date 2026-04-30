<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 | Updated: 2026-04-24 -->

# deve-note plan

## Purpose

Comprehensive engineering blueprint for Deve-Notebook. `docs/plan/` defines how the system is engineered; product-visible behavior lives in `docs/features/`, and automation-oriented validation lives in `docs/acceptance-cases/`.

## Key Files

| File | Description |
|------|-------------|
| `deve-note plan.md` | Master plan overview and table of contents |
| `01_terminology.md` | Core terms: note, vault, ledger, actor, fact, projection |
| `02_positioning.md` | Product positioning and target audience |
| `03_rendering.md` | Markdown rendering pipeline and extensions |
| `04_storage.md` | Ledger-first storage, node-first model, projection system |
| `05_network.md` | P2P sync protocol, WebSocket transport, transfer engine |
| `06_repository.md` | UUID-first repo identity, multi-repo catalog, shadow branches |
| `07_diff_logic.md` | Source control diff, rename tracking, target resolution |
| `08_ui_design.md` | UI design overview |
| `08_ui_design_01_web.md` | Web UI — layout, components, responsive design |
| `08_ui_design_02_desktop.md` | Desktop UI — native integration |
| `08_ui_design_03_mobile.md` | Mobile UI — touch gestures, drawers |
| `09_auth.md` | Authentication, E2E encryption, key exchange |
| `10_ai_agent.md` | Native AI chat baseline and trusted external agent boundary |
| `11_i18n.md` | Internationalization strategy |
| `12_commands.md` | Command palette and keyboard shortcuts |
| `13_settings.md` | Settings system and persistence |
| `14_tech_stack.md` | Technology choices and rationale |
| `15_release.md` | Build, packaging, and deployment |
| `16_web_thin_client_ledger.md` | Web thin client, repo-scoped state machine, scope gates |
| `17_plugins.md` | Trusted agent / calculation runtime interface reservation |
| `验收清单.md` | Acceptance checklist (Chinese) — **deprecated**, see `docs/acceptance-cases/` |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `plugins/` | Plugin system design documents (referenced from `10_ai_agent.md` Metadata) |

## For AI Agents

### Working In This Directory

- **Read before implementing.** Every feature should trace back to a plan chapter.
- Plans are written in Chinese and English. Key architectural concepts are defined in `01_terminology.md`.
- Critical design patterns: Route 2 (node-first), UUID-first identity, fail-closed semantics, scope nonces.
- `docs/features/` contains Chrome MCP manual walkthroughs for user-visible behavior; do not move that content back into plan chapters.
- `docs/acceptance-cases/` contains automation-oriented cases; keep those scripts and control-surface checks separate from plan prose.
- Do not modify plan files unless asked — they are reference documents.

## Plan-Code Bijection Enforcement (双射执行机制)

Plan 与代码必须保持强制对应关系。本机制分三层落地：

### Layer 1 — Plan Reference Annotations (代码侧注解)

每个实现 plan 条款的 Rust 模块 **MUST** 在文件头包含 `plan_ref:` 注解，指向权威 plan 章节与子章节：

```rust
//! plan_ref:
//!   - 04_storage#watcher-contract
```

**规则**：
- 注解格式为 `//! plan_ref:` 紧接 YAML-ish 列表；每行一条，格式 `  - <chapter_basename>#<stable-anchor-id>`。
- `#` 后面是 plan 章节里用 `{#id}` 声明的稳定 anchor，**MUST NOT** 依赖自然语言标题文字。
- 纯工具/util 模块（如 `utils/path.rs`）可使用 `//! plan_ref: infra` 标记为基础设施，豁免章节追溯。
- 同一模块 MAY 引用多个章节；跨域模块应优先拆分而非堆叠引用。
- 删除代码前 MUST 核对其 `plan_ref` 对应条款是否已从 plan 中移除或重新分配。
- 新增 plan_ref 时 MUST 在 plan 章节相应节加上 `{#anchor-id}`；若无 anchor，MUST 先补 anchor 再写代码引用。

**稳定 plan anchor registry**：

本表记录 `docs/plan/` 中可被代码 `plan_ref` 引用的稳定锚点。锚点出现在本表不代表当前必须已有代码引用；是否已被实现覆盖以 `scripts/plan-coverage.sh` 的反向覆盖矩阵为准。

| Anchor | Plan 位置 | 语义 |
|---|---|---|
| `03_rendering#markdown-render-whitelist` | `### 4.3 Whitelist Rule` | Markdown 渲染白名单、HTML 过滤与安全链接边界 |
| `03_rendering#large-document-runtime` | `## 7. Large Document Strategy` | 大文档、UTF-16 index cache 与渲染/runtime 定位策略 |
| `03_rendering#document-authority-bridge` | `### 12.4 Authority Bridge` | 文档 snapshot/history/edit/ack/reject 权威桥接合同 |
| `04_storage#facts-partition` | `### 2.3 Facts Partition` | Content Facts / Structure Facts 与 LedgerEvent 权威模型 |
| `04_storage#projection-contract` | `## 7. Projection and Persistence Contract` | 投影与持久化合同（drift detection、projection writeback） |
| `04_storage#watcher-contract` | `## 8. Watcher Contract` | 文件监听合同（watcher、pending_fs） |
| `04_storage#backup-export` | `### 9.4 Backup / Export` | 备份与灾备导出（JSONL export） |
| `04_storage#repo-runtime-layout` | `### 3.2 Repo Runtime Layout` | `.notegit`、repo runtime metadata 与内部目录布局 |
| `04_storage#browser-storage-layering` | `### 3.4 Browser Storage Layering` | 浏览器 localStorage/IndexedDB/WebCrypto 分层与降级合同 |
| `04_storage#internal-path-normalization` | `### 3.5 Internal Path Normalization` | ledger/projection/sync payload 路径 forward-slash 规范化 |
| `05_network#server-ws-runtime` | `### 12.3 Server WS Runtime` | Server WebSocket runtime、sync handler 与 scoped outbound 合同 |
| `05_network#web-ws-runtime` | `### 12.4 Web Runtime` | WebSocket/API client runtime、握手与消息同步合同 |
| `06_repository#repo-catalog-contract` | `### 3.3 Catalog Rule` | local/remote repo catalog 作为 selector/listing 输入层的 fail-closed 合同 |
| `06_repository#repo-catalog-repair-contract` | `### 7.2 Catalog Repair` | repo catalog metadata/name/url/file-stem 修复与隔离合同 |
| `06_repository#repo-selector-resolution-contract` | `### 2.5 Selector Inputs and Logical Identity` | UUID-first selector 解析、别名恢复与歧义 fail-closed 合同 |
| `06_repository#tree-projection-contract` | `## 5. Tree Projection Contract` | Structure Facts 到 tree projection 的权威与修复合同 |
| `06_repository#repo-scope-runtime` | `### 9.3 Scope Runtime Layer` | repo/branch/scope_nonce 运行时隔离与 fail-closed 合同 |
| `07_diff_logic#source-control-runtime` | `### 9.3 Server Runtime` | Source-control WS/HTTP handler 运行时 |
| `08_ui_design_01_web#single-binary-distribution` | `## 2. Single Binary Distribution` | Web 静态资源构建、托管与 SPA fallback 合同 |
| `09_auth#auth-http-endpoints` | `### 4.1 HTTP Endpoints` | login/logout/status/me HTTP endpoint 合同 |
| `09_auth#jwt-cookie-contract` | `## 5. JWT and Cookie Contract` | JWT claims、签发/验证、cookie 交付合同 |
| `09_auth#password-hashing` | `### 5.5 Password Hashing` | Argon2 PHC 密码哈希与验证合同 |
| `09_auth#auth-rate-limiting` | `### 6.4 Rate Limiting` | 登录与连接限流合同 |
| `09_auth#security-headers` | `### 6.5 Security Headers` | HTTP 安全头合同 |
| `09_auth#session-probe-policy` | `## 7. Session Probe Policy` | `/api/auth/status` 前台 session probe 与后台暂停合同 |
| `09_auth#unauthorized-handling` | `### 9.1 Unauthorized Handling` | `401/403/AUTH_*` 进入 Unauthorized 并退出写态 |
| `09_auth#unauthorized-disconnected-ui` | `### 9.4 Unauthorized vs Disconnected UI Contract` | Unauthorized 与 Disconnected 的 UI/重连分流合同 |
| `09_auth#auth-config` | `## 本章相关配置` | 鉴权环境变量 |
| `10_ai_agent#native-ai-chat-runtime` | `## 2. Native AI Chat` | Native AI Chat server/UI/streaming bridge 的 read-first 运行时合同 |
| `10_ai_agent#trusted-agent-bridge` | `## 3. Trusted External Agent Bridge` | Trusted CLI Agent 的 default-off、policy-gated 桥接合同 |
| `12_commands#cli-commands` | `## 1. CLI Commands` | CLI 命令集合、帮助面与配置命令入口 |
| `12_commands#command-palette-shortcuts` | `## 2. Command Palette` | Command Palette、Quick Open 与全局快捷键入口 |
| `13_settings#configuration-settings` | `## 2. Configuration Settings (config.toml)` | `config.toml` 运行时配置读取/写入合同 |
| `13_settings#keyboard-shortcuts` | `## 3. Keyboard Shortcuts` | 用户可见快捷键映射合同 |
| `13_settings#browser-ui-prefs` | `## 4. Browser UI Preferences` | 浏览器本地 UI 偏好持久化与敏感数据禁止边界 |
| `14_tech_stack#search-baseline` | `### 1.2 搜索基线` | repo-scoped baseline search、可禁用索引与 Tantivy feature-gated 实现 |
| `15_release#runtime-observability` | `### 5.4 Runtime Observability` | 运行时状态、连接角色与 release/debug 可观测性 |
| `16_web_thin_client_ledger#web-edit-intent` | `### 4.1 Edit Intent` | Web thin client 写意图、writer identity 与服务端权威提交边界 |
| `17_plugins#skills-cli-extension-boundary` | `### MCP Retirement Boundary` | MCP 退役后 Skills + 受控 CLI 扩展边界 |
| `17_plugins#plugin-runtime-boundary` | `## 2. Existing Rhai Plugin Host Boundary` | 外围 Rhai/plugin-host/PluginCall 兼容运行时边界，禁止升级为默认插件平台 |

### Layer 2 — CI Coverage Check (覆盖率扫描)

`scripts/plan-coverage.sh` 扫描 `crates/` 与 `apps/` 下所有 `.rs` 文件，输出：
1. 无 `plan_ref` 注解的非测试源码模块计数（warning，非阻塞）
2. 引用了已不存在的章节或章节名的模块清单（error，阻塞）
3. plan 章节的反向覆盖矩阵：每个 `§section` 被哪些代码文件引用

默认输出保持 CI 友好的计数与反向覆盖矩阵；需要处理 `plan_ref` 债务时，可运行 `scripts/plan-coverage.sh --summary-missing-plan-ref` 输出聚合分布，或运行 `scripts/plan-coverage.sh --list-missing-plan-ref` 输出非豁免 missing 模块路径清单。

测试文件、test support、bench、generated/vendor/dist/public glue 不计入缺失注解 warning；但这些文件一旦声明 `plan_ref`，仍会参与 dangling 校验和反向覆盖矩阵。普通 `src/bin` 和 runtime support 文件不默认豁免。

CI 流水线 MUST 运行此脚本；产出的 `plan-coverage.txt` 作为 PR artifact 留存。

### Layer 3 — Acceptance Case Binding (验收用例绑定)

`docs/acceptance-cases/` 下每个验收用例文件 `ACC-XXX.md` MUST 对应至少一个集成测试函数，命名模式：

```rust
#[test]
fn acc_xxx_<slug>() { ... }
```

`scripts/plan-coverage.sh` 同时扫描 acceptance case 文件名与测试函数名，输出未绑定测试的用例清单。

### Minimum Automated Checks (最小强制检查)

CI MUST 同时运行：
- 单文件行数检查：`crates/` 与 `apps/` 下 `.rs` 文件超过 250 行为软架构警告，超过 500 行才阻塞（熔断阈值）
- i18n facade 检查：`apps/web/src/components/` 下硬编码中/英文用户可见字符串即阻塞

以上检查统一封装于 `scripts/plan-coverage.sh`，单入口执行全部验证。

<!-- MANUAL: -->

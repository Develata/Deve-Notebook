<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 | Updated: 2026-06-04 -->

# deve-note plan

## Purpose

Comprehensive engineering blueprint for Deve-Notebook. `docs/plan/` defines how the system is engineered; product-visible behavior lives in `docs/features/`, and automation-oriented validation lives in `docs/acceptance-cases/`.

## Key Files

| File | Description |
|------|-------------|
| `deve-note plan.md` | Master plan overview and table of contents |
| `01_terminology.md` | Core terms: ledger, projection, writer gate, pending overlay, WebLightPeer, scope nonce |
| `02_positioning.md` | Product positioning and target audience |
| `10_rendering.md` | Markdown rendering pipeline and extensions |
| `03_storage/` | Ledger-first storage, node-first model, projection system |
| `07_network.md` | P2P sync protocol, WebSocket transport, transfer engine |
| `04_repository.md` | UUID-first repo identity, multi-repo catalog, shadow branches |
| `05_diff_logic.md` | Source control diff, rename tracking, target resolution |
| `11_ui_design/` | Shared UI shell/control/runtime topology and native adapter gate registry |
| `11_ui_design/01_web.md` | Web UI — layout, components, responsive design |
| `11_ui_design/02_desktop.md` | Desktop UI — native integration |
| `11_ui_design/03_mobile.md` | Mobile UI — touch gestures, drawers |
| `08_auth.md` | Authentication, E2E encryption, key exchange |
| `16_ai_agent.md` | Native AI chat baseline and trusted external agent boundary |
| `13_i18n.md` | Internationalization strategy and authoritative error code catalog |
| `14_commands.md` | Command palette and keyboard shortcuts |
| `15_settings.md` | Settings system and persistence |
| `17_tech_stack.md` | Technology choices and rationale |
| `18_release.md` | Build, packaging, and deployment |
| `09_web_thin_client_ledger.md` | Web thin client, pending overlay, repo-scoped writer gate, ack/reject contract |
| `19_plugins.md` | Trusted agent / calculation runtime interface reservation |
| `06_backup.md` | Branch-scoped backup / restore locator, encrypted pack, WebDAV and S3 boundary |
| `12_source_control_ui.md` | VS Code-like Source Control view contract and boundary |
| `../registry/runtime-skeleton-registry.md` | Runtime convergence status and current module path registry |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `plugins/` | Plugin system design documents (referenced from `16_ai_agent.md` Metadata) |

## For AI Agents

### Working In This Directory

- **Read before implementing.** Every feature should trace back to a plan chapter.
- Plans are written in Chinese and English. Key architectural concepts are defined in `01_terminology.md`.
- Critical design patterns: Route 2 (node-first), UUID-first identity, fail-closed semantics, scope nonces.
- `docs/features/` contains Chrome MCP manual walkthroughs for user-visible behavior; do not move that content back into plan chapters.
- `docs/acceptance-cases/` contains automation-oriented cases; keep those scripts and control-surface checks separate from plan prose.
- Do not modify plan files unless asked — they are reference documents.
- `13_i18n.md#i18n-error-code-catalog` is the only authoritative error-code catalog; other chapters may classify failure domains but must not define parallel error-code lists.
- `05_diff_logic.md#git-mirror-lifecycle` is the authoritative Git mirror lifecycle and command-boundary contract; storage and command chapters should reference it instead of duplicating preflight/import/export/push rules.
- `pending overlay` is Web thin-client session runtime state; it must not be modeled as `pending_fs_ops` or cleared by watcher/scan semantics.
- `11_ui_design/index.md#native-post-gate-common-contract` owns shared Desktop/Mobile post-gate native shell requirements; Desktop/Mobile subchapters should contain only platform deltas.
- `docs/registry/runtime-skeleton-registry.md` owns the current Runtime Skeleton Registry status/path table; new refactor targets should be added there or explicitly marked as local-only before appearing in chapter tails.
- 修改任一 plan 章节内容（typo 以上级别）后 MUST 刷新该章 Metadata 的 `Last Review` 字段；`Last Review` 仅由 plan review 行为更新，不与 git commit 时间挂钩。
- Metadata 的 `Version` 仅表示章节自身版本（骨架级修改才 bump major）；MUST NOT 与 release version、protocol version (`WS_PROTOCOL_VERSION`)、redb schema version 或 HTTP API version 混用。`scripts/plan-coverage.sh --check-metadata-completeness` 强制校验每章 `Version` / `Last Review` 字段存在。

## Plan-Code Bijection Enforcement (双射执行机制)

Plan 与代码必须保持强制对应关系。本机制分三层落地：

### Layer 1 — Plan Reference Annotations (代码侧注解)

每个实现 plan 条款的 Rust 模块 **MUST** 在文件头包含 `plan_ref:` 注解，指向权威 plan 章节与子章节：

```rust
//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
```

**规则**：
- 注解格式为 `//! plan_ref:` 紧接 YAML-ish 列表；每行一条，格式 `  - <chapter-path>#<stable-anchor-id>`。
- `<chapter-path>` 可以是单文件章节的 basename（如 `04_repository`），也可以是多文件章节的相对路径（如 `03_storage/authority`，对应 `docs/plan/03_storage/authority.md`）。chapter-path 只允许一层子文件，不得出现多级子目录。
- `#` 后面是 plan 章节里用 `{#id}` 声明的稳定 anchor，**MUST NOT** 依赖自然语言标题文字。
- 纯工具/util 模块（如 `utils/path.rs`）可使用 `//! plan_ref: infra` 标记为基础设施，豁免章节追溯。
- 同一模块 MAY 引用多个章节；跨域模块应优先拆分而非堆叠引用。
- 删除代码前 MUST 核对其 `plan_ref` 对应条款是否已从 plan 中移除或重新分配。
- 新增 plan_ref 时 MUST 在 plan 章节相应节加上 `{#anchor-id}`；若无 anchor，MUST 先补 anchor 再写代码引用。

**chapter-path 兼容窗口**：`scripts/plan-coverage.sh` 同时接受 basename 与 chapter-path 两种形式，互不冲突。当多文件章节拆分时，旧 basename anchor 到新 chapter-path anchor 的批量迁移由 `scripts/plan-coverage.sh --rewrite-plan-ref --from <旧前缀> --to <新前缀> [--apply]` 完成（默认 dry-run，仅 `--apply` 才写文件；只改 `//! plan_ref:` 块内列表项前缀，保留注释前缀、缩进与行尾注释）。

**稳定 plan anchor registry**：

本表记录 `docs/plan/` 中可被代码 `plan_ref` 引用的稳定锚点。锚点出现在本表不代表当前必须已有代码引用；是否已被实现覆盖以 `scripts/plan-coverage.sh` 的反向覆盖矩阵为准。

| Anchor | Plan 位置 | 语义 |
|---|---|---|
| `10_rendering#markdown-render-whitelist` | `### 4.3 Whitelist Rule` | Markdown 渲染白名单、HTML 过滤与安全链接边界 |
| `10_rendering#link-activation-gate` | `### 5.2 Link Activation` | Ctrl/Cmd 链接激活闸门、全局 modifier state 与 guarded external open |
| `10_rendering#code-block-toolbar-contract` | `### 6.4 Code Block Toolbar Contract` | CodeMirror adapter 代码块 Copy/Ellipsis toolbar、可扩展菜单与空 action 状态; no-rust-plan-ref |
| `10_rendering#outline-projection` | `### 6.5 Outline Projection` | Outline heading scan、inline projection 与跳转语义 |
| `10_rendering#large-document-runtime` | `## 7. Large Document Strategy` | 大文档、UTF-16 index cache 与渲染/runtime 定位策略 |
| `10_rendering#document-authority-bridge` | `### 12.4 Authority Bridge` | 文档 snapshot/history/edit/ack/reject 权威桥接合同 |
| `03_storage/authority#facts-partition` | `authority.md ### 2.3 Facts Partition` | Content Facts / Structure Facts 与 LedgerEvent 权威模型 |
| `03_storage/authority#ledger-entry-format-contract` | `authority.md ### 4.1.1 Ledger Entry Format Contract` | LedgerEntry 序列化/解码格式与版本兼容合同 |
| `03_storage/authority#redb-schema-version-contract` | `authority.md ### 4.3.1 Redb Schema Version Gate` | redb schema 版本闸门与迁移/拒绝边界 |
| `03_storage/projection#projection-contract` | `projection.md ## 7. Projection and Persistence Contract` | 投影与持久化合同（drift detection、projection writeback） |
| `03_storage/watcher#watcher-contract` | `watcher.md ## 8. Watcher Contract` | 文件监听合同（watcher、pending_fs） |
| `03_storage/repair#backup-export` | `repair.md ### 9.4 Backup / Export` | 备份与灾备导出（JSONL export） |
| `03_storage/index#repo-runtime-layout` | `index.md ### 3.2 Repo Runtime Layout` | `.notegit`、repo runtime metadata 与内部目录布局 |
| `03_storage/projection#projection-locator-contract` | `projection.md ### 3.2.1 Projection Locator Layout` | repo-scoped projection base locator、computed workspace root 与冲突边界 |
| `03_storage/index#git-ecosystem-coexistence` | `index.md ### 3.2.2 Git Mirror Storage Boundary` | Git mirror 存储边界与 `.git` / `.notegit` 共存约束 |
| `03_storage/index#browser-storage-layering` | `index.md ### 3.4 Browser Storage Layering` | 浏览器 localStorage/IndexedDB/WebCrypto 分层与降级合同 |
| `03_storage/index#internal-path-normalization` | `index.md ### 3.5 Internal Path Normalization` | ledger/projection/sync payload 路径 forward-slash 规范化 |
| `07_network#full-peer-mesh-v1` | `### 3.1.1 Full Peer Mesh v1` | 静态 FullPeer mesh v1 的 shadow-only、显式合并与低资源边界 |
| `07_network#static-peer-config` | `### 3.1.2 Static Peer Configuration` | P2P 静态 peer 配置、token env 与 fail-closed 配置入口 |
| `07_network#full-peer-ws-admission` | `### 4.1.1 Full Peer /ws Admission` | Browser session 与 FullPeer bearer admission 的分离合同 |
| `07_network#repo-scoped-handshake` | `### 6.1 Repo-Scoped Handshake` | SyncHello、scope_nonce 与 repo-scoped handshake 合同 |
| `07_network#trust-boundary` | `### 10.2 Trust Boundary` | trust boundary policy；relay 只转发、写入路径由签名来源决定 |
| `07_network#remote-shadow-apply-atomicity` | `### 10.4 Remote Shadow Apply Atomicity` | remote shadow apply、snapshot replay 与 local write fault 的原子性/隔离合同 |
| `07_network#relay-proxy-attribution-contract` | `### 10.5 Indirect Sync and Attribution` | relay/proxy 只转发、按 repo route 与 payload source peer 归属路由的合同 |
| `07_network#server-ws-runtime` | `### 12.3 Server WS Runtime` | Server WebSocket runtime、sync handler 与 scoped outbound 合同 |
| `07_network#web-ws-runtime` | `### 12.4 Web Runtime` | WebSocket/API client runtime、握手与消息同步合同 |
| `04_repository#repo-catalog-contract` | `### 3.3 Catalog Rule` | local/remote repo catalog 作为 selector/listing 输入层的 fail-closed 合同 |
| `04_repository#repo-catalog-repair-contract` | `### 7.2 Catalog Repair` | repo catalog metadata/name/url/file-stem 修复与隔离合同 |
| `04_repository#repo-health-and-repair` | `## 7. Recovery / Repair Contract` | repo degraded/repair/quarantine 与 projection repair 的恢复合同 |
| `04_repository#repo-selector-resolution-contract` | `### 2.5 Selector Inputs and Logical Identity` | UUID-first selector 解析、别名恢复与歧义 fail-closed 合同 |
| `04_repository#tree-projection-contract` | `## 5. Tree Projection Contract` | Structure Facts 到 tree projection 的权威与修复合同 |
| `04_repository#repo-scope-runtime` | `### 9.3 Scope Runtime Layer` | repo/branch/scope_nonce 运行时隔离与 fail-closed 合同 |
| `05_diff_logic#authority-diff-core` | `### 2.3 Authority Rule` | diff / stage / merge 最终收敛到 ledger facts 的 authority 边界 |
| `05_diff_logic#git-mirror-lifecycle` | `### 2.3.1 Git Mirror Lifecycle` | Git mirror readiness、import/export/push 与只读 status 边界 |
| `05_diff_logic#source-control-runtime` | `### 9.3 Server Runtime` | Source-control WS/HTTP handler 运行时 |
| `05_diff_logic#merge-contract` | `### 3.3 Merge Lifecycle` | MergePeer / ResolveMergeConflict 的同 repo、冲突检测与确认输出合同 |
| `11_ui_design/index#layout-navigation-and-focus` | `### 5.2 Focus State` | layout shell 的 focus trap、focus restore 与跨 surface focus state 合同 |
| `11_ui_design/index#editor-group-tabstrip` | `### 3.6 Editor Group Tabs` | 主编辑区 doc/diff tab strip 的 view-local 状态、切换、关闭与 authority 边界 |
| `11_ui_design/index#context-action-surface` | `### 3.3.1 Context Action Surface` | file tree/menu/command/shortcut 的 ContextAction 投影、执行边界与 external action provenance 合同 |
| `11_ui_design/index#native-adapter-gate-registry` | `### 8.5 Native Adapter Gate Registry` | Desktop/Mobile native adapter 的 authority gate、no-packaging-runtime 默认构建与子章权限边界 |
| `11_ui_design/index#native-post-gate-common-contract` | `### 8.6 Native Post-Gate Common Contract` | Desktop/Mobile post-gate 共用 service boot、本地通信、adapter feature scope 与性能预算合同 |
| `11_ui_design/01_web#single-binary-distribution` | `## 1. Single Binary Distribution` | Web 静态资源构建、托管与 SPA fallback 合同 |
| `11_ui_design/01_web#web-layout-persistence` | `## 6. Resizable Layout` | Web 布局尺寸、面板持久化与 local UI prefs 边界 |
| `11_ui_design/02_desktop#desktop-current-native-boundary` | `## 1. 原生适配器边界` | Desktop native adapter 当前边界与 post-gate 目标区分（check-native-track-boundary.sh 断言）; no-rust-plan-ref |
| `11_ui_design/02_desktop#desktop-native-adapter-contract` | `### 1.1 Minimal Native Adapter Contract` | Desktop native adapter 的最小 endpoint/session/bootstrap/readiness 合同 |
| `11_ui_design/02_desktop#desktop-native-shell-modes` | `### 1.1.1 Desktop Native Shell Modes` | Desktop `NativeShellMode` 的 `LocalBackend` / `RemoteBrowser` 语义、sidecar/loopback/session handoff 与 remote preference 探测边界 |
| `11_ui_design/02_desktop#desktop-packaging-scaffold` | `### 1.2 Desktop Packaging Scaffold` | Desktop packaging scaffold 与 no-packaging skeleton 边界 |
| `11_ui_design/02_desktop#desktop-packaging-dependency-gate-decision` | `### 1.3 Desktop Packaging Dependency Gate` | Desktop native-packaging dependency spike 决策与默认关闭边界 |
| `11_ui_design/02_desktop#desktop-service-supervisor-contract` | `### 1.4 Embedded Service Supervisor Contract` | Desktop embedded service supervisor 状态机与 readiness 分类 |
| `11_ui_design/02_desktop#desktop-process-adapter-decision` | `### 1.5 Process Adapter Gate` | Desktop process adapter gate 的诊断、authority 与 packaging 前置条件 |
| `11_ui_design/03_mobile#mobile-current-native-boundary` | `## 1. 原生适配器边界` | Mobile native adapter 当前边界与 post-gate 目标区分 |
| `11_ui_design/03_mobile#mobile-native-adapter-contract` | `### 1.1 Minimal Native Adapter Contract` | Mobile native adapter 的最小 endpoint/session/bootstrap/readiness 合同 |
| `11_ui_design/03_mobile#mobile-native-shell-modes` | `### 1.1.1 Mobile Native Shell Modes` | Mobile `NativeShellMode` 的 embedded loopback/session bootstrap、WebView readiness 与 `RemoteBrowser` 边界 |
| `11_ui_design/03_mobile#mobile-service-supervisor-contract` | `### 1.2 Embedded Service Supervisor Contract` | Mobile embedded service supervisor、foreground reprobe 与 suspension 边界 |
| `11_ui_design/03_mobile#mobile-process-adapter-decision` | `### 1.3 Process Adapter Gate` | Mobile process adapter gate 的诊断、authority 与 runtime 前置条件 |
| `11_ui_design/03_mobile#mobile-packaging-scaffold` | `### 1.4 Mobile Packaging Scaffold` | Mobile packaging scaffold 与 no-packaging skeleton 边界（check-native-track-boundary.sh 断言）; no-rust-plan-ref |
| `11_ui_design/03_mobile#mobile-packaging-dependency-gate-decision` | `### 1.5 Mobile Packaging Dependency Gate` | Mobile native-packaging dependency spike 决策与默认关闭边界 |
| `11_ui_design/03_mobile#mobile-android-shell-package-execution-gate` | `### 1.6 Android Shell-only Package Execution Gate` | Android shell-only package execution 的 target-host、authority 与 process-runtime 门禁 |
| `11_ui_design/03_mobile#mobile-ios-shell-package-execution-gate` | `### 1.7 iOS Shell-only Package Execution Gate` | iOS shell-only package execution 的 target-host、authority 与 process-runtime 门禁 |
| `11_ui_design/03_mobile#mobile-responsive-layout` | `## 2. Responsive Architecture` | Mobile responsive layout、drawer 与 safe-area shell 合同 |
| `11_ui_design/03_mobile#mobile-interaction-design` | `## 3. Interaction Design` | Mobile gesture、touch target、toolbar 与 interaction affordance 合同 |
| `11_ui_design/03_mobile#mobile-surface-switcher` | `### 5.4.1 Mobile Surface Switcher` | Mobile document/diff surface switcher、touch-safe bottom sheet 与 shared tab runtime 合同 |
| `08_auth#auth-http-endpoints` | `### 4.1 HTTP Endpoints` | login/logout/status/me HTTP endpoint 合同 |
| `08_auth#jwt-cookie-contract` | `## 5. JWT and Cookie Contract` | JWT claims、签发/验证、cookie 交付合同 |
| `08_auth#password-hashing` | `### 5.5 Password Hashing` | Argon2 PHC 密码哈希与验证合同 |
| `08_auth#cors` | `### 6.2 CORS` | CORS origin allowlist、wildcard 禁止与 credential boundary |
| `08_auth#auth-rate-limiting` | `### 6.4 Rate Limiting` | 登录与连接限流合同 |
| `08_auth#security-headers` | `### 6.5 Security Headers` | HTTP 安全头合同 |
| `08_auth#audit` | `### 6.6 Audit` | 鉴权、安全事件与审计记录边界 |
| `08_auth#key-and-file-permissions` | `### 6.7 Key and File Permissions` | identity key 与本地文件权限 fail-closed 合同 |
| `08_auth#localhost-dev-policy` | `### 6.8 Localhost / Dev Policy` | localhost/dev 例外、匿名访问与 loopback 限定规则 |
| `08_auth#session-probe-policy` | `## 7. Session Probe Policy` | `/api/auth/status` 前台 session probe 与后台暂停合同 |
| `08_auth#unauthorized-handling` | `### 9.1 Unauthorized Handling` | `401/403/AUTH_*` 进入 Unauthorized 并退出写态 |
| `08_auth#unauthorized-disconnected-ui` | `### 9.4 Unauthorized vs Disconnected UI Contract` | Unauthorized 与 Disconnected 的 UI/重连分流合同 |
| `08_auth#auth-config` | `## 本章相关配置` | 鉴权环境变量 |
| `16_ai_agent#native-ai-chat-runtime` | `## 2. Native AI Chat` | Native AI Chat server/UI/streaming bridge 的 read-first 运行时合同 |
| `16_ai_agent#trusted-agent-bridge` | `## 3. Trusted External Agent Bridge` | Trusted CLI Agent 的 default-off、policy-gated 桥接合同 |
| `13_i18n#i18n-facade-contract` | `## 1. Internationalization Strategy` | `t::*` facade、用户可见文案与协议文案分层 |
| `13_i18n#i18n-resource-management` | `## 2. Resource Management` | locale resource 加载、fallback 与资源组织 |
| `13_i18n#i18n-keys-reference` | `## 5. I18n Keys Reference` | i18n key namespace 与引用表 |
| `13_i18n#i18n-error-code-catalog` | `## 6. Error Code Catalog` | 后端结构化错误码到前端文案的唯一权威目录 |
| `14_commands#cli-commands` | `## 1. CLI Commands` | CLI 命令集合、帮助面与配置命令入口 |
| `14_commands#command-palette-shortcuts` | `## 2. Command Palette` | Command Palette、Quick Open 与全局快捷键入口 |
| `15_settings#configuration-settings` | `## 2. Configuration Settings (config.toml)` | `config.toml` 运行时配置读取/写入合同 |
| `15_settings#native-host-local-backend-preference` | `### 2.2.2 Native Host-local Backend Preference` | Desktop/Mobile native backend preference 的 host-local JSON 持久化、local/remote 模式与敏感数据禁止边界 |
| `15_settings#keyboard-shortcuts` | `## 3. Keyboard Shortcuts` | 用户可见快捷键映射合同 |
| `15_settings#browser-ui-prefs` | `## 4. Browser UI Preferences` | 浏览器本地 UI 偏好持久化与敏感数据禁止边界 |
| `17_tech_stack#graph-visualization` | `### 1.1 图谱可视化` | Graph visualization baseline 与 graph projection 技术边界 |
| `17_tech_stack#search-baseline` | `### 1.2 搜索基线` | repo-scoped baseline search、可禁用索引与 Tantivy feature-gated 实现 |
| `17_tech_stack#native-packaging-dependency-gate` | `### 1.4 原生打包依赖门禁` | Desktop/Mobile native-packaging optional dependency 与 gate policy |
| `17_tech_stack#performance-profiles-and-feature-matrix` | `## 3. Performance Profiles & Feature Matrix` | profile 枚举（standard/low-spec）与 feature matrix 权威；op 维度 budget 归 21_perf_budget |
| `18_release#runtime-observability` | `### 5.4 Runtime Observability` | 运行时状态、连接角色与 release/debug 可观测性 |
| `09_web_thin_client_ledger#write-readiness` | `### 2.3 Write Readiness` | Web thin client repo-scoped 写入就绪状态合同 |
| `09_web_thin_client_ledger#web-edit-intent` | `### 4.1 Edit Intent` | Web thin client 写意图、writer identity 与服务端权威提交边界 |
| `19_plugins#skills-cli-extension-boundary` | `### MCP Retirement Boundary` | MCP 退役后 Skills + 受控 CLI 扩展边界 |
| `19_plugins#plugin-runtime-boundary` | `## 2. Existing Rhai Plugin Host Boundary` | 外围 Rhai/plugin-host/PluginCall 兼容运行时边界，禁止升级为默认插件平台 |
| `06_backup#backup-locator-contract` | `## 2. Locator Model` | repo/branch URL 扩展为 WebDAV/S3 backup locator 的绑定与 authority 边界 |
| `06_backup#backup-remote-layout-contract` | `## 6. Remote Layout` | repo manifest、branch manifest、packs prefix 与 remote layout drift 的结构化诊断边界 |
| `06_backup#backup-root-contract` | `### 3.1 Backup Root` | remote repo-level namespace 的 locator、expected RepoId、format_version、provider_kind 校验边界 |
| `06_backup#backup-branch-binding-contract` | `### 3.2 Branch Backup Binding` | branch/writer 到 backup folder/prefix 的 1:1 绑定、writable 与 active writer 冲突边界 |
| `06_backup#backup-pack-contract` | `### 3.3 Backup Pack` | backup pack manifest、ledger facts range、snapshot/blob refs 与完整性 hash 边界 |
| `06_backup#backup-upload-state-machine-contract` | `### 4.1 Backup Upload` | backup upload 从 BindingValidated 到 Complete 的状态推进、加密/上传/remote verify 顺序边界 |
| `06_backup#backup-restore-candidate-contract` | `### 3.4 Restore Candidate` | verify/decrypt 后的 restore candidate admission、RemoteReadonly / ExplicitImport / ExplicitMerge 边界 |
| `06_backup#backup-restore-state-machine-contract` | `### 4.2 Restore / Import` | backup restore 从 RemoteDiscovered 到 RestoreCandidate 的状态推进、下载阶段禁写 ledger 与显式写 gate 边界 |
| `06_backup#backup-secret-ref-contract` | `## 7. Security Contract` | backup credential/key 只能作为 env/keyring/config 引用进入 runtime，禁止裸 secret/token/key material |
| `06_backup#backup-verification-contract` | `## 7. Security Contract` | manifest/pack hash、认证证据、decrypt gate 与 RepoId 一致性的 fail-closed 校验边界 |
| `06_backup#backup-artifact-protection-contract` | `## 7. Security Contract` | manifest/pack 上传前加密与认证 metadata admission，key ref 只作为引用进入 runtime |
| `06_backup#backup-provider-dispatch-contract` | `### 10.1 Backup Runtime` | WebDAV/S3 provider adapter dispatch、credential/key ref 接入与 provider metadata 非权威边界 |
| `06_backup#backup-command-output-contract` | `### 5.3 Outputs` | BackupBindingStatus / BackupPlan / BackupError 的命令可见结构化输出与 fail-closed 分类边界 |
| `12_source_control_ui#source-control-vscode-reference-contract` | `## 2. Reference Policy` | VS Code-like SCM mental model、reference baseline 与禁止复制实现资产边界 |
| `12_source_control_ui#external-changes-sibling-view` | `## 4.1 External Changes Sibling View` | External Changes 同级入口、投影偏差导入 ledger 与 Source Control commit anchor 分离边界 |
| `20_operations_catalog#opid-catalog` | `## 1. Scope & Authority` | operation-flow 目录唯一权威（OpId catalog）; planned/no-code-yet |
| `20_operations_catalog#extension-point-index` | `## 4. Extension Point Index` | 暴露给 plugins/host 的扩展点索引; planned/no-code-yet |
| `20_operations_catalog#replacement-point-index` | `## 5. Replacement Point Index` | feature-flag 可替换点索引; planned/no-code-yet |
| `20_operations_catalog#configuration-entry-index` | `## 6. Configuration Entry Index` | 配置入口主索引（定义 defer 各原章）; planned/no-code-yet |
| `21_perf_budget#critical-path-budget` | `## 2. Critical Path Budget` | 关键路径 P50/P99 latency 与 RSS budget 表; planned/no-code-yet |
| `21_perf_budget#perf-budget-fuse` | `## 3. CI Fuse Thresholds` | CI fuse 阈值；由 scripts/plan-coverage.sh --check-perf-budget enforcing（shell 合同，无 Rust plan_ref）; no-rust-plan-ref |
| `22_reliability_observability#slo-sli-catalog` | `## 2. SLO / SLI Catalog` | SLO/SLI 目标与 Error Budget; planned/no-code-yet |
| `22_reliability_observability#telemetry-schema` | `## 3. Telemetry Schema` | 结构化日志/事件字段标准; planned/no-code-yet |
| `22_reliability_observability#metrics-taxonomy` | `## 4. Metrics Taxonomy` | counter/gauge/histogram 命名与维度规则; planned/no-code-yet |
| `22_reliability_observability#tracing-span-boundary` | `## 5. Tracing Span Boundary` | Flow Coordination root span 边界; planned/no-code-yet |
| `22_reliability_observability#observation-to-health-mapping` | `## 6. Observation-to-Health Mapping` | 观测信号→04 health 状态映射（状态全集 defer 04）; planned/no-code-yet |
| `22_reliability_observability#alerting-tier` | `## 7. Alerting Tier` | 错误码/health 信号→告警等级映射（错误码 defer 13）; planned/no-code-yet |
| `22_reliability_observability#dr-playbook-index` | `## 8. DR Playbook Index` | 灾难恢复手册索引（步骤 defer 06）; planned/no-code-yet |
| `23_threat_model#trust-boundaries` | `## 2. Trust Boundaries` | STRIDE 分析的信任边界引用（定义 defer 07）; planned/no-code-yet |
| `23_threat_model#stride-catalog` | `## 3. STRIDE Catalog` | STRIDE 威胁面与缓解归属目录; planned/no-code-yet |
| `23_threat_model#key-lifecycle` | `## 4. Key Lifecycle (高层流程)` | 密钥生命周期高层流程（具体协议 defer 08/06/07）; planned/no-code-yet |
| `23_threat_model#algorithm-deprecation` | `## 5. Algorithm Deprecation` | 加密原语退役策略与迁移窗口; planned/no-code-yet |
| `23_threat_model#supply-chain` | `## 6. Supply Chain` | SBOM/reproducible build/dependency gate/signing 策略; planned/no-code-yet |
| `23_threat_model#coordinated-vulnerability-disclosure` | `## 7. Coordinated Vulnerability Disclosure` | CVD 渠道/embargo/SLA 策略; planned/no-code-yet |

### Layer 2 — CI Coverage Check (覆盖率扫描)

`scripts/plan-coverage.sh` 扫描 `crates/` 与 `apps/` 下所有 `.rs` 文件，输出：
1. 无 `plan_ref` 注解的非测试源码模块计数（warning，非阻塞）
2. 引用了已不存在的章节或章节名的模块清单（error，阻塞）
3. plan 章节的反向覆盖矩阵：每个 `§section` 被哪些代码文件引用

默认输出保持 CI 友好的计数与反向覆盖矩阵；需要处理 `plan_ref` 债务时，可运行 `scripts/plan-coverage.sh --summary-missing-plan-ref` 输出聚合分布，或运行 `scripts/plan-coverage.sh --list-missing-plan-ref` 输出非豁免 missing 模块路径清单。

测试文件、test support、bench、generated/vendor/dist/public glue 不计入缺失注解 warning；但这些文件一旦声明 `plan_ref`，仍会参与 dangling 校验和反向覆盖矩阵。普通 `src/bin` 和 runtime support 文件不默认豁免。

CI 流水线 MUST 运行此脚本；产出的 `plan-coverage.txt` 作为 PR artifact 留存。

**最终强制门禁（B4 后，CI/发布逐项运行，任一失败即阻断）**：除默认报告（`blocking violations: 0`，含 dangling/registry/inline-header 守卫）外，以下子命令均为 enforcing：

- `--check-reverse-coverage`：每个 stable registry anchor 必须至少有一条代码侧 `plan_ref`；标 `planned/no-code-yet`（先登记后落地）或 `no-rust-plan-ref`（shell/脚本断言、无 Rust plan_ref）的 anchor 跳过。
- `--check-metadata-completeness`：每个 `docs/plan/NN_*` 章节 Metadata 声明 `Version` + `Last Review`（缺整块 Metadata 亦失败）。
- `--check-perf-budget`：`21_perf_budget` §2 预算表已写入数值（非 TBD）。
- `--check-no-adr-plan-ref`：无 `plan_ref` 指向 ADR（`docs/adr/` 是 decision-history slice，不被 plan_ref 引用）。
- `--check-md-links [dirs...]`：`docs/plan` / `docs/features` / `docs/acceptance-cases` 内相对 markdown 链接与 `#anchor` 全部解析。
- `scripts/plan-coverage-selftest.sh`：脚本自身单元自测。

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

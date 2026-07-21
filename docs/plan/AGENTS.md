<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 | Updated: 2026-07-19 -->

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
| `06_backup.md` | Projection Backup: WebDAV/S3 Markdown Projection Workspace file transport |
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
- 测试、生成源码与真正 repo-local infra 只能通过 `scripts/plan-ref-exemptions.tsv` 的**精确文件路径 + 类别 + 理由**显式豁免；不得再依赖文件名、目录名或行数启发式自动跳过。
- repo-local infra 模块同时 MUST 使用精确 `//! plan_ref: infra` header，并在 exemption registry 中登记为 `local-only-infra`；`infra` 只表示分类，不能自行充当无理由豁免。
- `generated` 豁免的文件 MUST 带明确的 generated / do-not-edit provenance marker；手写文件名包含 `generated` 不构成豁免依据。
- 同一模块 MAY 引用多个章节；跨域模块应优先拆分而非堆叠引用。
- 删除代码前 MUST 核对其 `plan_ref` 对应条款是否已从 plan 中移除或重新分配。
- 新增 plan_ref 时 MUST 在 plan 章节相应节加上 `{#anchor-id}`；若无 anchor，MUST 先补 anchor 再写代码引用。

**chapter-path 兼容窗口**：`scripts/plan-coverage.sh` 同时接受 basename 与 chapter-path 两种形式，互不冲突。当多文件章节拆分时，旧 basename anchor 到新 chapter-path anchor 的批量迁移由 `scripts/plan-coverage.sh --rewrite-plan-ref --from <旧前缀> --to <新前缀> [--apply]` 完成（默认 dry-run，仅 `--apply` 才写文件；只改 `//! plan_ref:` 块内列表项前缀，保留注释前缀、缩进与行尾注释）。

**稳定 plan anchor registry**：

本表记录 `docs/plan/` 中可被代码 `plan_ref` 引用的稳定锚点。锚点出现在本表不代表当前必须已有代码引用；是否已被实现覆盖以 `scripts/plan-coverage.sh` 的反向覆盖矩阵为准。

<!-- stable-plan-anchor-registry:start -->
| Anchor | Plan 位置 | 语义 |
|---|---|---|
| `01_terminology#normative-language` | `## 1. Normative Language` | MUST/SHOULD/MAY 规范词义与合同解释边界 |
| `01_terminology#core-definitions` | `## 2. Core Definitions` | ledger/projection/runtime/authority 等核心术语唯一语义 |
| `02_positioning#core-boundaries` | `## 3. Core Boundaries` | 首发核心 MUST/MUST NOT、插件可选面与产品定位边界 |
| `10_rendering#current-rendering-split` | `## 1.1 Rendering Capability Boundary` | baseline/extended rendering 能力分层与 source-first projection 边界 |
| `10_rendering#markdown-render-whitelist` | `### 4.3 Whitelist Rule` | Markdown 渲染白名单、HTML 过滤与安全链接边界 |
| `10_rendering#link-activation-gate` | `### 5.2 Link Activation` | Ctrl/Cmd 链接激活闸门、全局 modifier state 与 guarded external open |
| `10_rendering#code-block-toolbar-contract` | `### 6.4 Code Block Toolbar Contract` | CodeMirror adapter 代码块 Copy/Ellipsis toolbar、可扩展菜单与空 action 状态 |
| `10_rendering#outline-projection` | `### 6.5 Outline Projection` | Outline heading scan、inline projection 与跳转语义 |
| `10_rendering#large-document-runtime` | `## 7. Large Document Strategy` | 大文档、UTF-16 index cache 与渲染/runtime 定位策略 |
| `10_rendering#document-authority-bridge` | `### 12.4 Authority Bridge` | 文档 snapshot/history/edit/ack/reject 权威桥接合同 |
| `03_storage/authority#facts-partition` | `authority.md ### 2.3 Facts Partition` | Content Facts / Structure Facts 与 LedgerEvent 权威模型 |
| `03_storage/authority#ledger-entry-format-contract` | `authority.md ### 4.1.1 Ledger Entry Format Contract` | LedgerEntry 序列化/解码格式与版本兼容合同 |
| `03_storage/authority#redb-schema-version-contract` | `authority.md ### 4.3.1 Redb Schema Version Gate` | redb schema 版本闸门与迁移/拒绝边界 |
| `03_storage/authority#repo-mutation-publication-gate` | `authority.md ### 6.1.1 Repository Mutation Publication Gate` | repo-scoped 本地 authority writer 串行、提交结果分类与有序 projection recovery 发布 |
| `03_storage/authority#local-authority-owner-contract` | `authority.md #### 11.1.1 Local Authority Owner Contract` | per-RepoId Redb owner、prepared admission、不可 Clone lease、quiesce/retirement 与 persistent lock 合同 |
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
| `07_network#projection-recovery-contract` | `### 4.3.3 Projection Recovery Wire Contract` | scoped typed projection recovery、后端刷新计划与 External Apply ack/receipt wire |
| `07_network#repo-control-wire-contract` | `### 4.3.2 Repo Control Wire Contract` | F4/v5 nested host-local alias/create 与 ownership-aware removal Prepare/Execute projection |
| `07_network#native-full-peer-runtime` | `### 12.5 Native Full Peer Runtime` | CLI/native FullPeer transport、repo-scoped admission 与宿主 runtime 边界 |
| `04_repository#repo-catalog-contract` | `### 3.3 Catalog Rule` | local/remote repo catalog 作为 selector/listing 输入层的 fail-closed 合同 |
| `04_repository#repo-catalog-repair-contract` | `### 7.2 Catalog Repair` | repo catalog metadata/name/url/file-stem 修复与隔离合同 |
| `04_repository#repo-health-and-repair` | `## 7. Recovery / Repair Contract` | repo degraded/repair/quarantine 与 projection repair 的恢复合同 |
| `04_repository#repo-selector-resolution-contract` | `### 2.5 Selector Inputs and Logical Identity` | UUID-first selector 解析、别名恢复与歧义 fail-closed 合同 |
| `04_repository#host-repo-alias-contract` | `### 2.1.1 Host-local Repo Alias Contract` | host-local alias ownership、CAS、JSON import/export 与跨宿主禁止边界；alias core/CLI/F4-v4 path已承载，F4/v5 removal不改变alias authority |
| `04_repository#local-repo-removal-contract` | `### 7.6 Local Repo Removal Contract` | ownership manifest、confirmation token、authority retirement、owner cleanup、repair与NoScope合同 |
| `04_repository#repo-lifecycle-coordinator` | `### 7.9 Repo Lifecycle Coordinator` | host-owned lifecycle job、prepare/cut/settle 与 immutable committed/settled plans |
| `04_repository#tree-projection-contract` | `## 5. Tree Projection Contract` | Structure Facts 到 tree projection 的权威与修复合同 |
| `04_repository#repo-scope-runtime` | `### 9.3 Scope Runtime Layer` | repo/branch/scope_nonce 运行时隔离与 fail-closed 合同 |
| `05_diff_logic#authority-diff-core` | `### 2.3 Authority Rule` | diff / stage / merge 最终收敛到 ledger facts 的 authority 边界 |
| `05_diff_logic#git-mirror-lifecycle` | `### 2.3.1 Git Mirror Lifecycle` | Git mirror readiness、import/export/push 与只读 status 边界 |
| `05_diff_logic#remote-projection-transport` | `### 2.3.2 Remote Projection Transport Contract` | 过渡锚点：当前 push/pull 实现与批准 Route B transport 边界；保留既有代码反向覆盖 |
| `03_storage/index#remote-import-runtime-layout` | `index.md ### 3.1.1 Remote Import Runtime Layout` | Remote Import host-only manifest/blob/candidate 布局；B1 artifact runtime 已落地 |
| `03_storage/authority#remote-import-workflow-tables` | `authority.md ### 4.3.2 Remote Import Workflow Tables` | Redb v4 session/runtime tables、active pointer 与 terminal retention；B1 durable store 已落地 |
| `03_storage/authority#projection-fault-recovery-table` | `authority.md ### 4.3.3 Projection Fault Recovery Table` | Redb v4 local-authority profile 的 repo-local、host-only Projection Fault side table |
| `03_storage/authority#sealed-ledger-change-batch` | `authority.md ### 6.3.1 Sealed Prepared Ledger Change Batch` | source-specific sealed batch 与单事务 authority apply |
| `03_storage/projection#remote-import-projection-writeback` | `projection.md ### 7.1 Remote Import Projection Writeback` | Ledger commit 后的 projection writeback 与 degraded receipt |
| `03_storage/projection#durable-projection-fault-contract` | `projection.md ### 7.2 Durable Projection Fault Store` | repo-local Redb v4 Projection Fault recovery evidence、typed origin 与 Remote Import receipt 原子收敛 |
| `03_storage/repair#remote-import-cleanup-repair` | `repair.md ### 9.4.1 Remote Import Cleanup Repair` | cleanup_pending、orphan 与 repair；B1 dry-run inventory 已落地，B3 已持久化 Applied cleanup debt，产品 repair/收敛待 B4 |
| `04_repository#remote-import-repo-lifecycle` | `### 7.6.1 Remote Import Repo Lifecycle` | RepoId 绑定、rename/remove 与 provider generation 协调 |
| `05_diff_logic#remote-import-diff-contract` | `### 2.3.3 Remote Import Diff Contract` | immutable candidate、opaque entry_id 与 backend-owned diff |
| `06_backup#remote-projection-transport-contract` | `## 3. Remote Projection Transport Contract` | B2 已落地 WebDAV/S3 push、profile/credential/HTTP/signing 与 ordered source acquisition 的共享 host transport；产品 Prepare 接线待 B4 |
| `06_backup#remote-import-session-contract` | `## 4. Immutable Remote Import Session` | B1 已落地 immutable session identity/snapshot；B3 已落地内部 Applied Pending receipt 与 exactly-once replay，产品接线/Projection outcome 收敛待 B4 |
| `06_backup#remote-import-state-machine` | `### 4.1 State Machine` | B1 已落地 Preparing/Ready/Stale/Failed/Discarded；B3 已落地内部 Applied transition 与 cleanup debt，产品恢复路径待 B4 |
| `06_backup#remote-import-removal-owner-plan` | `### 4.1.1 Repo Removal Owner Plan` | Remote Import状态复核、immutable owner cleanup plan与artifact-only removal边界 |
| `06_backup#remote-import-resource-contract` | `### 4.2 Resource Contract` | B1 已落地 capture 文件、字节与路径预算；wire 分页预算待 B4 |
| `06_backup#remote-import-runtime-boundary` | `## 10. Runtime Boundary` | B1 已落地 store/capture，B2 已落地 transport 依赖边界，B3 已落地 sealed authority writer；产品接线待 B4，UI 待 B5 |
| `06_backup#projection-backup-failure-modes` | `## 8. Failure Modes` | Remote Projection transport 与 Remote Import capture/session/apply 的 typed failure、fail-closed 与 cleanup 边界 |
| `07_network#remote-import-wire-contract` | `### 4.3.1 Remote Import Wire Contract` | F4/v5 nested request/response、typed errors 与不泄密投影 |
| `09_web_thin_client_ledger#remote-import-client-contract` | `### 11.4 Remote Import Client Contract` | scope/revision-bound typed client 与 stale response 丢弃；planned/no-code-yet |
| `09_web_thin_client_ledger#repo-control-client-contract` | `### 11.5 Repo Control Client Contract` | exact RepoId/alias revision/job status thin client |
| `12_source_control_ui#remote-import-sibling-view` | `## 4.2 Remote Import Sibling View` | Remote Import 与 Source Control/External Changes 同级但独立的薄壳层；planned/no-code-yet |
| `14_commands#remote-import-command-contract` | `### 1.1 Remote Projection Push and Remote Import Commands` | projection push 与 remote-import prepare/review/apply/manage 命令面 |
| `05_diff_logic#typed-diff-projection-contract` | `### 2.5 Typed Diff Projection Contract` | backend-owned typed diff projection、显示层消费与 authority-neutral diff 边界 |
| `05_diff_logic#source-control-runtime` | `### 9.3 Server Runtime` | Source-control WS/HTTP handler 运行时 |
| `05_diff_logic#merge-contract` | `### 3.3 Merge Lifecycle` | MergePeer / ResolveMergeConflict 的同 repo、冲突检测与确认输出合同 |
| `11_ui_design/index#layout-navigation-and-focus` | `### 5.2 Focus State` | layout shell 的 focus trap、focus restore 与跨 surface focus state 合同 |
| `11_ui_design/index#layout-tokens-and-layer-registry` | `### 3.4 Layout Tokens and Layer Registry` | canonical semantic color、z-index 与 shared layout token registry |
| `11_ui_design/index#ui-runtime-boundary` | `## 10. Runtime Boundary` | view/control/runtime 依赖方向与薄前端壳层边界 |
| `11_ui_design/index#editor-group-tabstrip` | `### 3.6 Editor Group Tabs` | 主编辑区 doc/diff tab strip 的 view-local 状态、切换、关闭与 authority 边界 |
| `11_ui_design/index#context-action-surface` | `### 3.3.1 Context Action Surface` | file tree/menu/command/shortcut 的 ContextAction 投影、执行边界与 external action provenance 合同 |
| `11_ui_design/index#native-adapter-gate-registry` | `### 8.5 Native Adapter Gate Registry` | Desktop/Mobile native adapter 的 authority gate、no-packaging-runtime 默认构建与子章权限边界 |
| `11_ui_design/index#native-post-gate-common-contract` | `### 8.6 Native Post-Gate Common Contract` | Desktop/Mobile post-gate 共用 service boot、本地通信、adapter feature scope 与性能预算合同 |
| `11_ui_design/01_web#single-binary-distribution` | `## 1. Single Binary Distribution` | Web 静态资源构建、托管与 SPA fallback 合同 |
| `11_ui_design/01_web#web-layout-persistence` | `## 6. Resizable Layout` | Web 布局尺寸、面板持久化与 local UI prefs 边界 |
| `11_ui_design/02_desktop#desktop-current-native-boundary` | `## 1. 原生适配器边界` | Desktop native adapter 当前边界与 post-gate 目标区分（deve_baseline native-track-boundary 断言） |
| `11_ui_design/02_desktop#desktop-native-adapter-contract` | `### 1.1 Minimal Native Adapter Contract` | Desktop native adapter 的最小 endpoint/session/bootstrap/readiness 合同 |
| `11_ui_design/02_desktop#desktop-native-shell-modes` | `### 1.1.1 Desktop Native Shell Modes` | Desktop `NativeShellMode` 的 `LocalBackend` / `RemoteBrowser` 语义、sidecar/loopback/session handoff 与 remote preference 探测边界 |
| `11_ui_design/02_desktop#desktop-packaging-scaffold` | `### 1.2 Desktop Packaging Scaffold` | Desktop packaging scaffold 与 no-packaging skeleton 边界 |
| `11_ui_design/02_desktop#desktop-packaging-dependency-gate-decision` | `### 1.3 Desktop Packaging Dependency Gate` | Desktop native-packaging dependency spike 决策与默认关闭边界 |
| `11_ui_design/02_desktop#desktop-service-supervisor-contract` | `### 1.4 Embedded Service Supervisor Contract` | Desktop embedded service supervisor 状态机与 readiness 分类 |
| `11_ui_design/02_desktop#desktop-process-adapter-decision` | `### 1.5 Process Adapter Gate` | Desktop process adapter gate 的诊断、authority 与 packaging 前置条件 |
| `11_ui_design/03_mobile#mobile-current-native-boundary` | `## 1. 原生适配器边界` | Mobile native adapter 当前边界与 post-gate 目标区分（deve_baseline native-track-boundary 断言） |
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
| `08_auth#local-cli-proxy-authority` | `### 6.11 Local CLI Proxy Authority` | server-held DB 的 loopback-only JWT admission 与 exact Remote Import capability |
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
| `14_commands#repo-alias-command-contract` | `### 1.0.1 Repo Alias Command Contract` | host-local alias JSON CLI、dry-run、warning summary 与 atomic accepted batch |
| `14_commands#command-palette-shortcuts` | `## 2. Command Palette` | Command Palette、Quick Open 与全局快捷键入口 |
| `15_settings#configuration-settings` | `## 2. Configuration Settings (config.toml)` | `config.toml` 运行时配置读取/写入合同 |
| `15_settings#native-host-local-backend-preference` | `### 2.2.2 Native Host-local Backend Preference` | Desktop/Mobile native backend preference 的 host-local JSON 持久化、local/remote 模式与敏感数据禁止边界 |
| `15_settings#keyboard-shortcuts` | `## 3. Keyboard Shortcuts` | 用户可见快捷键映射合同 |
| `15_settings#browser-ui-prefs` | `## 4. Browser UI Preferences` | 浏览器本地 UI 偏好持久化与敏感数据禁止边界 |
| `17_tech_stack#graph-visualization` | `### 1.1 图谱可视化` | Graph visualization baseline 与 graph projection 技术边界 |
| `17_tech_stack#search-baseline` | `### 1.2 搜索基线` | repo-scoped baseline search、可禁用索引与 Tantivy feature-gated 实现 |
| `17_tech_stack#git-ecosystem-bridge` | `### 1.3 Git 生态镜像桥` | Git executable / mirror bridge 只服务 ecosystem interop，不拥有 NoteGit authority |
| `17_tech_stack#native-packaging-dependency-gate` | `### 1.4 原生打包依赖门禁` | Desktop/Mobile native-packaging optional dependency 与 gate policy |
| `17_tech_stack#canonical-rust-toolchain` | `### Canonical Rust Toolchain` | workspace Rust toolchain pin、release baseline 与 target-host 一致性 |
| `17_tech_stack#performance-profiles-and-feature-matrix` | `## 3. Performance Profiles & Feature Matrix` | profile 枚举（standard/low-spec）与 feature matrix 权威；op 维度 budget 归 21_perf_budget |
| `18_release#runtime-observability` | `### 5.4 Runtime Observability` | 运行时状态、连接角色与 release/debug 可观测性 |
| `18_release#developer-baseline-checkers` | `### 2.1.1 Developer Baseline Checkers` | deve_baseline 确定性规则、TSV 执行器与无产品 authority 的治理边界 |
| `18_release#remote-browser-candidate-fixture` | `#### RemoteBrowser Candidate Fixture` | exact-HEAD RemoteBrowser target-host fixture、credential hygiene 与证据边界 |
| `18_release#first-tag-acceptance-matrix` | `### 2.1.4 First-tag Acceptance Matrix` | 首发 journey、producer/receipt 唯一性与 tag-ready 验收矩阵 |
| `18_release#artifact-identity-and-integrity` | `### 2.1.5 Artifact Identity and Integrity` | candidate artifact identity、checksums、SBOM、provenance 与 sealed bundle 合同 |
| `18_release#release-versioning` | `## 3. Versioning` | SemVer tag、Cargo/app version 与 release promotion 顺序合同 |
| `09_web_thin_client_ledger#write-readiness` | `### 2.3 Write Readiness` | Web thin client repo-scoped 写入就绪状态合同 |
| `09_web_thin_client_ledger#web-edit-intent` | `### 4.1 Edit Intent` | Web thin client 写意图、writer identity 与服务端权威提交边界 |
| `09_web_thin_client_ledger#projection-recovery-coordinator` | `### 8.1.1 Projection Recovery Coordinator` | Web generation-bound projection recovery、pending 保留、gap reconnect 与显式 Retry |
| `19_plugins#skills-cli-extension-boundary` | `### MCP Retirement Boundary` | MCP 退役后 Skills + 受控 CLI 扩展边界 |
| `19_plugins#plugin-runtime-boundary` | `## 2. Existing Rhai Plugin Host Boundary` | 外围 Rhai/plugin-host/PluginCall 兼容运行时边界，禁止升级为默认插件平台 |
| `06_backup#projection-backup-scope` | `## 1. Scope` | Remote Projection push 与 immutable Remote Import 属首发；Ledger history disaster recovery 属于非目标 |
| `06_backup#projection-backup-contract` | `## 2. Product Semantics` | push 只传输 Markdown projection；Remote Import 通过 immutable capture/review/sealed Ledger Apply，不覆盖 workspace 预审 |
| `06_backup#projection-backup-locator-contract` | `### 3.1 Locator and Profile Model` | Remote Projection locator/profile 只承载 secret-free locator 与 host-local credential binding |
| `06_backup#projection-backup-remote-layout-contract` | `### 3.2 Remote Object Layout` | provider remote layout 是 Markdown object set；Remote Import host artifacts 另归 repo/session identity layout |
| `06_backup#projection-backup-upload-state-machine-contract` | `### 3.3 Push State Machine` | push 只枚举并上传 Markdown projection files，不写 Ledger/Source Control authority |
| `06_backup#projection-backup-pull-state-machine-contract` | `## 11. Removed Pull Transition` | 负面合同：旧 pull→workspace→External Changes 已删除且不得恢复 |
| `06_backup#projection-backup-command-output-contract` | `## 6. Commands / Inputs / Outputs` | 正式命令面拆分 projection push 与 remote-import prepare/review/apply/manage，不保留 pull 命令 |
| `06_backup#projection-backup-secret-ref-contract` | `## 7. Security and Verification` | credential refs 归 host-local Remote Projection profile；不得进入 manifest/blob/wire/UI |
| `06_backup#projection-backup-verification-contract` | `## 7. Security and Verification` | capture/apply 重验 digest；Remote Import 证据不得由旧 pull tests 冒充 |
| `06_backup#projection-backup-provider-dispatch-contract` | `## 10. Runtime Boundary` | transport 只负责 push/source acquisition；session/review/apply 分别归 Remote Import runtime、sealed writer 与薄客户端 |
| `12_source_control_ui#source-control-vscode-reference-contract` | `## 2. Reference Policy` | VS Code-like SCM mental model、reference baseline 与禁止复制实现资产边界 |
| `12_source_control_ui#external-changes-sibling-view` | `## 4.1 External Changes Sibling View` | External Changes 同级入口、投影偏差导入 ledger 与 Source Control commit anchor 分离边界 |
| `20_operations_catalog#opid-catalog` | `## 1. Scope & Authority` | operation-flow 目录唯一权威（OpId catalog）；由 deve_baseline architecture-registry 绑定 |
| `20_operations_catalog#extension-point-index` | `## 4. Extension Point Index` | 暴露给 plugins/host 的扩展点索引；由 deve_baseline architecture-registry 绑定 |
| `20_operations_catalog#replacement-point-index` | `## 5. Replacement Point Index` | feature-flag 可替换点索引；由 deve_baseline architecture-registry 绑定 |
| `20_operations_catalog#configuration-entry-index` | `## 6. Configuration Entry Index` | 配置入口主索引（定义 defer 各原章）；由 deve_baseline architecture-registry 绑定 |
| `21_perf_budget#critical-path-budget` | `## 2. Critical Path Budget` | 关键路径 P50/P99 latency 与 RSS budget 表；由 deve_baseline perf-budget / PERF-001 绑定 |
| `21_perf_budget#perf-budget-fuse` | `## 3. CI Fuse Thresholds` | CI fuse 阈值；由 scripts/plan-coverage.sh --check-perf-budget enforcing（shell 合同，无 Rust plan_ref）; no-rust-plan-ref |
| `22_reliability_observability#slo-sli-catalog` | `## 2. SLO / SLI Catalog` | SLO/SLI 目标与 Error Budget；由 deve_baseline reliability-observability / REL-013 绑定 |
| `22_reliability_observability#telemetry-schema` | `## 3. Telemetry Schema` | 结构化日志/事件字段标准；由 deve_baseline reliability-observability / REL-013 绑定 |
| `22_reliability_observability#metrics-taxonomy` | `## 4. Metrics Taxonomy` | counter/gauge/histogram 命名与维度规则；由 deve_baseline reliability-observability / REL-013 绑定 |
| `22_reliability_observability#tracing-span-boundary` | `## 5. Tracing Span Boundary` | Flow Coordination root span 边界；由 deve_baseline reliability-observability / REL-013 绑定 |
| `22_reliability_observability#observation-to-health-mapping` | `## 6. Observation-to-Health Mapping` | 观测信号→04 RepoHealth 映射及 watcher failure 不映射为 RepoHealth/ProjectionFault 的正交边界；状态全集 defer 04、mount state defer 03 watcher；由 deve_baseline reliability-observability / REL-013 绑定 |
| `22_reliability_observability#alerting-tier` | `## 7. Alerting Tier` | 错误码/health 信号→告警等级映射（错误码 defer 13）；由 deve_baseline reliability-observability / REL-013 绑定 |
| `22_reliability_observability#resilience-playbook-index` | `## 8. Resilience Playbook Index` | 投影传输与 repo health 修复索引（步骤 defer 06/04）；由 deve_baseline reliability-observability / REL-013 绑定 |
| `23_threat_model#trust-boundaries` | `## 2. Trust Boundaries` | STRIDE 分析的信任边界引用（定义 defer 07）；治理策略合同，无 Rust plan_ref; no-rust-plan-ref |
| `23_threat_model#stride-catalog` | `## 3. STRIDE Catalog` | STRIDE 威胁面与缓解归属目录；治理策略合同，无 Rust plan_ref; no-rust-plan-ref |
| `23_threat_model#key-lifecycle` | `## 4. Key Lifecycle (高层流程)` | 密钥生命周期高层流程（具体协议 defer 08/06/07）；由 auth/security baseline 绑定 |
| `23_threat_model#algorithm-deprecation` | `## 5. Algorithm Deprecation` | 加密原语退役策略与迁移窗口；由 auth/security baseline 绑定 |
| `23_threat_model#supply-chain` | `## 6. Supply Chain` | SBOM/reproducible build/dependency gate/signing 策略；由 release-audit/auth baseline 绑定 |
| `23_threat_model#coordinated-vulnerability-disclosure` | `## 7. Coordinated Vulnerability Disclosure` | CVD 渠道/embargo/SLA 策略；由 SECURITY/auth baseline 绑定 |
<!-- stable-plan-anchor-registry:end -->

### Layer 2 — CI Coverage Check (覆盖率扫描)

`scripts/plan-coverage.sh` 扫描 `apps/`、`crates/` 与 `tools/` 下所有 present、非 ignored `.rs` 文件，输出：
1. 无规范 `plan_ref` 且没有精确显式豁免的源码模块清单（error，阻塞）
2. 引用了已不存在的章节或章节名的模块清单（error，阻塞）
3. plan 章节的反向覆盖矩阵：每个 `§section` 被哪些代码文件引用

默认输出保持 CI 友好的计数与反向覆盖矩阵；需要处理 `plan_ref` 债务时，可运行 `scripts/plan-coverage.sh --summary-missing-plan-ref` 输出聚合分布，或运行 `scripts/plan-coverage.sh --list-missing-plan-ref` 输出非豁免 missing 模块路径清单。

`scripts/plan-ref-exemptions.tsv` 是唯一豁免输入。每行必须声明一个 Rust 文件的精确 repo-relative path、`test` / `generated` / `local-only-infra` 类别、owner（generated 必须指向 tracked producer，其余为 `-`）和非空理由；`test` 还必须位于明确的 test/bench/test-support surface。重复、越界、缺失、陈旧、类别/header 不一致或 generated provenance 缺失均为 blocking。任何文件一旦声明真实 `plan_ref`，对应豁免必须删除；普通 `src/bin`、runtime support、短文件以及仅因名称含 `test` / `generated` 的文件均不默认豁免。

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

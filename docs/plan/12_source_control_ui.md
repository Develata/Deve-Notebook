# 12_source_control_ui.md - Source Control View Contract

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Version`: `0.0.1`
- `Last Review`: `2026-07-18`
- `Counterpart Feature`: `docs/features/07_diff_logic.md`, `docs/features/08_ui_design_02_desktop.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/04_diff.md`, `docs/acceptance-cases/05_ui.md`
- `Primary Code Areas`: `apps/web/src/components/sidebar/source_control/`, `apps/web/src/hooks/use_core/`

本章定义 Source Control view 及其同级 External Changes / Remote Import review surface 的显示层
合同。Source Control 以 VS Code SCM view 的信息架构和交互模型为长期参照，但另外两个 sibling
只复用合适的 UI primitive，不复制 VS Code implementation、DOM、CSS、图标资产或 Git authority。

Reference baseline 位于 `docs/reference/source-control/vscode-scm-baseline/`。该目录只保存版本化参考笔记、截图清单与交互抽象，不是权威实现。

## 1. Scope

本章只处理 Source Control view 及其同级 External Changes、Remote Import view 的显示层合同：

1. primary flow 的布局顺序。
2. resource groups、row actions 与 menu placement。
3. readonly / blocked / CLI-only notices 的可见状态。
4. history / graph 与 primary commit flow 的优先级。
5. 外部投影文件夹修改与 Source Control commit anchor 的显示边界。
6. immutable Remote Import review 与另外两个 change domain 的隔离边界。

非目标：

- 不重新定义 `05_diff_logic.md` 的 stage / commit / diff / merge authority。
- 不把 Git index、Git refs 或 `.git` object store 作为 Deve authority。
- 不复制 VS Code 源码、DOM、CSS、品牌图标或产品截图作为实现输入。
- 不要求 Web 直接执行 Git import / push / repair。
- 不把 External Changes 的 `Apply to Ledger` 解释为 Source Control commit。
- 不把 Remote Import Prepare/Review/Apply 解释为 Remote Projection pull、External Changes stage 或
  Source Control commit。

## 2. Reference Policy {#source-control-vscode-reference-contract}

Source Control view **MUST** 采用 VS Code-like SCM mental model：

- top-level commit box
- explicit resource groups
- row-level stage / unstage / discard affordances
- click-to-diff behavior
- secondary history / graph surfaces
- provider/repository context that does not dominate single-repo flow

该对齐是信息架构与交互模型对齐，不是视觉像素复制。

Reference baseline 规则：

- reference notes **MUST** 记录 VS Code version、OS、theme、capture date 与 source links。
- screenshots **MAY** 存档为参考证据，但不得成为实现 authority。
- reference notes **MUST** 抽象成 Deve terms：repo scope、branch role、writer gate、source-control runtime。
- VS Code product-specific services such as GitHub publish、extension marketplace、account integration 不进入 Deve Source Control contract。

## 3. View Structure

Source Control view 的默认顺序 **MUST** 是：

```text
Header
RepositoryContext
CommitInput
PrimaryCommitActions
BlockingNotice?
ConfirmedLedgerChanges
HistoryOrGraphSecondary
```

规则：

- `CommitInput` 是 top-level surface，不得嵌套在 `Changes` resource group 内。
- 单 repo 场景下 `RepositoryContext` **SHOULD** 保持紧凑或折叠。
- 多 repo / provider 场景下才允许展开 repositories list。
- `HistoryOrGraphSecondary` **SHOULD** 默认折叠或作为 secondary view；不得挤占 commit / confirmed changes primary flow。
- `RepositoryContext`、`History` 与 `Graph` 等可折叠 secondary panel header 必须使用真实 `button`
  语义并暴露 `aria-expanded` 与 `aria-controls`，被控制内容必须以稳定 `id` 常驻 DOM，并在折叠时
  使用 `hidden` 隐藏；它们只能展开/收起 view-local state，不得触发 source-control 写入。
- `Graph` 是只读 projection surface，不得写 ledger、workspace、source-control state 或 Git mirror state。

## 4. Resource Groups

Source Control 默认 resource groups：

- `Confirmed Ledger Changes`
- `Merge Changes` / conflict group（仅存在冲突时）
- optional read-only diagnostics group

规则：

- `Confirmed Ledger Changes` 必须显示 count，并明确其含义是已进入 ledger、尚未被最新 Source Control commit anchor 覆盖。
- 可折叠 resource group header 必须使用真实 `button` 语义并暴露 `aria-expanded` 与 `aria-controls`，
  被控制内容必须以稳定 `id` 常驻 DOM，并在折叠时使用 `hidden` 隐藏；header 右侧的
  section action 必须是独立按钮，不能嵌套在折叠按钮内，也不能触发展开/收起。
- `Confirmed Ledger Changes` 中的条目表示已进入 ledger、但未被最新 Source Control commit anchor 覆盖的 changes。
- 首版 `Confirmed Ledger Changes` 只能整体随 commit anchor 覆盖，不提供逐文件 include/exclude。
- Source Control **MUST NOT** 展示 External Changes 的 `Staged Changes` / `Changes` working groups。
- Source Control **MUST NOT** 对 `Confirmed Ledger Changes` 提供 Discard；未来撤回只能由独立 Revert flow 追加反向 ledger facts。
- conflict group 出现时必须优先展示，不能伪装成普通 modified row。
- empty state 必须说明当前 scope clean、readonly、blocked 或 unavailable，不能只显示空白。

## 4.1 External Changes Sibling View {#external-changes-sibling-view}

External Changes / 外部修改 是 Source Control 的同级入口，负责展示 Projection Workspace 与当前 ledger projection 的偏差。

默认顺序 **MUST** 是：

```text
Header
RepositoryContext?
BlockingNotice?
ApplyToLedgerAction
ExternalResourceGroups
```

默认 resource groups：

- `Staged External Changes`
- `External Changes`
- `Overlapping External Changes`（可作为行级状态或独立优先组）

规则：

- External Changes 只显示外部投影文件夹修改、watcher/scan 发现的偏差与对应 diff 入口。
- External Changes 支持 `Open Diff`、`Stage`、`Unstage`、`Discard External Change`、`Apply to Ledger` / `确认外部修改`。
- `Apply to Ledger` 按钮不得简称为 `Commit`；中文推荐文案是 `确认外部修改`，英文推荐文案是 `Apply to Ledger`。
- External Changes 不显示 history、graph、commit anchor 信息，也不创建 Source Control commit。
- External Changes 可共享 button、icon、row shell、section shell、touch target primitive；不得复用 Source Control 的业务 controller、commit controller、history/graph state、notice/error 语义。
- External Changes 的业务判断必须来自独立 domain/controller 或 backend/core typed state；view 层不得为了 UI 方便自行判定 ledger/source-control authority mutation。
- 如果 external change 与 confirmed ledger dirty 重叠，行必须显示 `与已确认账本更改重叠` / `Overlaps confirmed ledger changes`，
  禁用普通 `Stage` 与 `Apply to Ledger`，只允许 `Open Diff` 与 `Discard External Change`。

## 4.2 Remote Import Sibling View {#remote-import-sibling-view}

Remote Import 是 Source Control 与 External Changes 的第三个同级入口，负责审阅后端已封存的
immutable source snapshot 与 candidate revision。它不是远端文件浏览器，也不把内容预写入
Projection Workspace。

默认顺序 **MUST** 是：

```text
Header
RepositoryContext?
BlockingNotice?
SessionSummary
SessionActions
CandidateResourceGroup
```

规则：

- `SessionSummary` 只显示 backend 返回的 session state、revision、统计与泛化来源标签；不得显示
  locator、provider/host path、blob path、digest、credential、source manifest 或 raw failure detail。
- Candidate row 只使用 opaque strong `entry_id` 与 backend-generated display label。change kind 只包含
  `Added / Modified / Unchanged`；远端缺失项不显示为 Delete。
- blocker 与 change kind 正交。任何 blocker 都禁用整个 session Apply；view 不得从 label、detail、
  pending/staged 列表或 workspace 文件推断 blocker。
- row 点击只请求 typed diff；不得读取 blob、拼接 provider URL 或在浏览器计算 diff。
- `Refresh` 只请求后端从已封存 blobs 重算 candidate；`Apply` 与 `Discard` 都是 whole-session action。
- 首版没有 checkbox、逐文件 selection、逐文件 Apply、逐文件 Discard 或用 External Changes staging
  模拟 selection。
- Remote Import 可共享 button、icon、row shell、section shell、typed diff renderer 与 touch primitive；
  不得复用 Source Control/External Changes controller、state、notice/error 或 authority intent。
- Prepare 可在 Ledger readable 但 repo 未 Mounted 时执行；Apply 的 disabled state 只消费后端 typed
  blocker，未 Mounted 时显示 `STORAGE_WORKSPACE_INGESTION_UNAVAILABLE` 的本地化能力提示。

## 5. Row Interaction

每个 change row **MUST** 表达：

- status kind：modified / added / deleted / renamed / moved / conflict
- display path
- repository/branch scope when needed
- row-level primary action

交互规则：

- 点击 row 默认打开 diff。
- External Changes unstaged row 的 inline action 是 `Stage`；可提供 `Discard External Change`。
- External Changes staged row 的 inline action 是 `Unstage`。
- confirmed ledger row 的 inline action 只能是 `Open Diff`；不得使用 `Stage` / `Discard` / `Revert` 文案或语义；首版只提供打开 diff。
- Desktop row action tray 应采用 VS Code-like hover/focus affordance：row hover 或 keyboard focus 进入该行时显示 action buttons；coarse pointer / touch viewport 中 action buttons 必须保持可见且满足移动端触控尺寸。
- status kind 字母标记必须保留在 row 右侧独立 status slot；不得与 row action tray 混合，也不得覆盖或挤压 display path。
- External Changes section header 可提供 `Stage All` / `Unstage All` / `Discard All`；Source Control confirmed section 不提供这些动作。
- Remote Import candidate row 只提供 `Open Diff`；Refresh/Apply/Discard 位于 session action surface，
  不提供行级 mutation 或 checkbox。
- destructive actions 必须经各自 domain runtime gate；Source Control、External Changes 与 Remote Import
  不得借用彼此的 writer grant。必要时需要 explicit confirmation。
- remote readonly branch 中，row actions 必须 disabled 或替换为 read-only explanation。

## 6. Commit Surface

Commit surface 规则：

- commit message input **MUST** 位于 resource groups 之前。
- primary action 是 `Commit` 或当前 gate 允许的等价 commit action。
- `Commit & Push` 是 secondary action；它不得暗示 Web 直接拥有 Git push authority。
- AI-generated commit message 是辅助输入，必须服从 `16_ai_agent.md` 的 capability gate。
- message empty、staged empty and confirmed empty、readonly、writer-not-ready、scope switching、service offline 都必须显示结构化 disabled reason。
- commit button 在 confirmed ledger changes 非空时可用；confirmed-only commit 采用整锚提交。
- staged external changes 必须先通过 External Changes 的 `Apply to Ledger` 进入 ledger，之后才由 Source Control commit anchor 覆盖。

## 7. Menus and Commands

所有 action 必须映射到 stable command/control intent。
Source Control header 的 section visibility menu 必须是 button-driven：trigger 暴露
`aria-haspopup="menu"` 与 `aria-expanded`，menu item 暴露 checked state，选择任一 item
后自动关闭；该菜单只能切换 view-local section visibility，不得触发 source-control 写入。

推荐 command ids：

- `source_control.refresh`
- `source_control.stage`
- `source_control.stage_all`
- `source_control.unstage`
- `source_control.unstage_all`
- `source_control.discard`
- `source_control.commit`
- `source_control.open_diff`
- `source_control.open_history`
- `source_control.open_graph`

Future command id（首批不启用）：

- `source_control.revert_confirmed_ledger`

NoteGit/Git main mirror diagnostic command ids 必须保持独立，不得表示 Source Control authority：

- `ngit.mirror_status`
- `ngit.import_changes`
- `ngit.push_mirror`
- `ngit.repair_mirror`

Remote Projection push 与 Remote Import command ids 必须保持独立：

- `remote_projection.webdav.push`
- `remote_projection.s3.push`
- `remote_import.webdav.prepare`
- `remote_import.s3.prepare`
- `remote_import.open`
- `remote_import.refresh`
- `remote_import.apply`
- `remote_import.discard`

External Changes command ids 必须保持独立：

- `external_changes.refresh`
- `external_changes.stage`
- `external_changes.stage_all`
- `external_changes.unstage`
- `external_changes.unstage_all`
- `external_changes.discard`
- `external_changes.apply_to_ledger`
- `external_changes.open_diff`

Web ngit/Git-main-mirror diagnostic commands 当前只能打开 backend/runtime intent 或 read-only repair review；
不得直接升级为 Git writer。Remote Projection command 只执行 push；Remote Import commands 只发送
typed session intent。provider acquisition、immutable capture、candidate/blocker 计算与 Ledger Apply
均属于 backend/core infra。

旧 `webdav:push`、`webdav:pull`、`s3:push`、`s3:pull` 已由 B4 删除；它们不是 deprecated alias，
不得重新进入 Source Control registry。现有 Push 使用上列稳定 capability ids，Remote Import 使用独立 client。

## 8. Failure and Boundary States

Source Control view 必须显式展示以下状态：

- no repo selected
- repo switching
- branch switching
- remote readonly branch
- writer not ready
- service offline
- session invalid / unauthorized
- source-control runtime error
- Git main mirror queued / out-of-sync / read-only repair notice

External Changes view 必须显式展示以下状态：

- no repo selected
- repo switching / branch switching
- remote readonly branch
- writer not ready
- service offline
- projection workspace unavailable
- external change overlaps confirmed ledger changes
- external-change runtime error

Remote Import view 必须显式展示以下状态：

- no repo selected / scope transitioning
- no active session
- preparing / ready / stale / failed / applied / discarded
- active-session conflict
- typed whole-session blocker
- provider unavailable / limit exceeded / cleanup required
- Apply workspace ingestion unavailable
- post-commit Projection degraded receipt

状态来源必须是 runtime typed state 或 structured error code，不得由 view 层猜测。

## 9. Forbidden Patterns

- 把 `CommitInput` 嵌入 `Changes` group。
- 单 repo 默认展开 repositories list 并压过 commit flow。
- 默认展开 graph/history 并压过 staged/changes flow。
- 用 `git.*` command id 表示 Deve stage / commit authority。
- 让 view 组件直接改写 pending/staged side tables。
- 让 Source Control controller 同时管理 External Changes 业务状态。
- 让 Source Control 或 External Changes controller 同时管理 Remote Import session。
- 在 Source Control 中对 confirmed ledger row 暴露 Discard。
- 将 External Changes 的 `Apply to Ledger` 文案写成普通 `Commit`。
- 为 Remote Import 增加 checkbox、逐文件 Apply/Discard，或从 raw detail/path 推断 blocker。
- 把 Remote Import candidate 预写入 Workspace/External Changes 再要求用户确认。
- 复制 VS Code DOM/CSS/source files 作为 Deve UI implementation。
- 把 reference screenshots 当作 pixel-perfect requirement。

## 10. Runtime Boundary

### 10.1 View Layer

职责：

- render Source Control view
- render resource groups
- render row actions
- dispatch typed intents

### 10.2 Application Control

职责：

- command routing
- writer gate / readonly gate presentation
- scope-safe action dispatch

### 10.3 Source Control Runtime

职责：

- request confirmed ledger changes
- commit anchor creation
- history / commit diff
- structured notices

### 10.4 External Changes Runtime

职责：

- request external pending / staged state
- stage / unstage / discard external changes
- apply staged external changes to ledger
- detect overlap with confirmed ledger dirty and fail-closed
- structured external-change notices

### 10.5 Remote Import Client Runtime

职责：

- request session list/show/page/diff and whole-session refresh/apply/discard
- bind every request to request/repo/branch/scope; install Prepared/List results by the special rules in
  `07_network#remote-import-wire-contract`, then bind selected state to exact session/revision and discard stale response
- render backend-owned typed blockers/change kinds/diff projections
- surface durable Apply receipt and post-commit Projection degraded outcome

它不得拥有 provider transport、captured blobs、candidate computation、blocker inference、Ledger writer
或 cleanup authority。

View layer 不得直接操作 repo state、ledger state、pending/staged side tables、Git mirror state 或 `.notegit`。
View layer 也不得直接枚举、上传、下载或覆盖 WebDAV/S3 projection files。

## 11. Refactor Target

Source Control UI 应继续归属：

- `ui_shell`
- `application_control`
- `feature_runtime`
- `source_control_runtime`
- `diff_session_runtime`

External Changes UI 应登记为独立 `external_changes_client` / runtime facade，归属：

- `ui_shell`
- `application_control`
- `feature_runtime`
- `external_changes_runtime`
- `diff_session_runtime`

Remote Import UI 应登记为独立 `remote_import_client`，归属：

- `ui_shell`
- `application_control`
- `feature_runtime`
- backend `remote_import_runtime` 的 typed projection facade
- shared typed diff renderer（只读）

`remote_import_client` 仍为未启动，B5 才允许签署收敛。B4 已删除 Remote Projection command 打开
Source Control 与使用 `SourceControlNotice` 的路径；缺失期间不得恢复为 adapter。

若未来新增 `source_control_view_runtime`，必须先登记到 `docs/registry/runtime-skeleton-registry.md`，并说明它只拥有 view-local state。

## 本章相关命令

- `Source Control: Refresh`
- `Source Control: Stage All`
- `Source Control: Commit`
- `Source Control: Open History`
- `Source Control: Open Graph`
- `Remote Projection: WebDAV Push`
- `Remote Projection: S3 Push`
- `Remote Import: WebDAV Prepare`
- `Remote Import: S3 Prepare`
- `Remote Import: Open`
- `Remote Import: Refresh`
- `Remote Import: Apply`
- `Remote Import: Discard`

## 本章相关配置

- `source_control.show_repositories`
- `source_control.show_graph`
- `source_control.show_history`

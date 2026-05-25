# 12_source_control_ui.md - Source Control View Contract

## Metadata

- `Layer`: `Application / UI Shell`
- `Status`: `Current UI Contract`
- `Version`: `0.0.1`
- `Last Review`: `2026-05-24`
- `Counterpart Feature`: `docs/features/07_diff_logic.md`, `docs/features/08_ui_design_02_desktop.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/04_diff.md`, `docs/acceptance-cases/05_ui.md`
- `Primary Code Areas`: `apps/web/src/components/sidebar/source_control/`, `apps/web/src/hooks/use_core/`

本章定义 Source Control view 的显示层合同。它以 VS Code SCM view 的信息架构和交互模型为长期参照，但不复制 VS Code implementation、DOM、CSS、图标资产或 Git authority。

Reference baseline 位于 `docs/reference/source-control/vscode-scm-baseline/`。该目录只保存版本化参考笔记、截图清单与交互抽象，不是权威实现。

## 1. Scope

本章只处理 Source Control view 的 view contract：

1. primary flow 的布局顺序。
2. resource groups、row actions 与 menu placement。
3. readonly / blocked / CLI-only notices 的可见状态。
4. history / graph 与 primary commit flow 的优先级。

非目标：

- 不重新定义 `05_diff_logic.md` 的 stage / commit / diff / merge authority。
- 不把 Git index、Git refs 或 `.git` object store 作为 Deve authority。
- 不复制 VS Code 源码、DOM、CSS、品牌图标或产品截图作为实现输入。
- 不要求 Web 直接执行 Git import / push / repair。

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
ResourceGroups
HistoryOrGraphSecondary
```

规则：

- `CommitInput` 是 top-level surface，不得嵌套在 `Changes` resource group 内。
- 单 repo 场景下 `RepositoryContext` **SHOULD** 保持紧凑或折叠。
- 多 repo / provider 场景下才允许展开 repositories list。
- `HistoryOrGraphSecondary` **SHOULD** 默认折叠或作为 secondary view；不得挤占 commit / changes primary flow。
- `Graph` 是只读 projection surface，不得写 ledger、workspace、source-control state 或 Git mirror state。

## 4. Resource Groups

默认 resource groups：

- `Staged Changes`
- `Changes`
- `Merge Changes` / conflict group（仅存在冲突时）
- optional read-only diagnostics group

规则：

- `Staged Changes` 与 `Changes` 必须可见区分，并显示 count。
- `Changes` 中的条目表示 pending / working changes。
- `Staged Changes` 中的条目表示即将进入 commit 的 staged entries。
- conflict group 出现时必须优先展示，不能伪装成普通 modified row。
- empty state 必须说明当前 scope clean、readonly、blocked 或 unavailable，不能只显示空白。

## 5. Row Interaction

每个 change row **MUST** 表达：

- status kind：modified / added / deleted / renamed / moved / conflict
- display path
- repository/branch scope when needed
- row-level primary action

交互规则：

- 点击 row 默认打开 diff。
- unstaged row 的 inline action 是 `Stage`；可提供 `Discard`。
- staged row 的 inline action 是 `Unstage`。
- section header 可提供 `Stage All` / `Unstage All` / `Discard All`。
- destructive actions 必须经 source-control runtime gate；必要时需要 explicit confirmation。
- remote readonly branch 中，row actions 必须 disabled 或替换为 read-only explanation。

## 6. Commit Surface

Commit surface 规则：

- commit message input **MUST** 位于 resource groups 之前。
- primary action 是 `Commit` 或当前 gate 允许的等价 commit action。
- `Commit & Push` 是 secondary action；它不得暗示 Web Git mirror push authority。
- AI-generated commit message 是辅助输入，必须服从 `16_ai_agent.md` 的 capability gate。
- message empty、staged empty、readonly、writer-not-ready、scope switching、service offline 都必须显示结构化 disabled reason。

## 7. Menus and Commands

所有 action 必须映射到 stable command/control intent。

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

Git mirror command ids 必须保持独立：

- `git.import_changes`
- `git.push_mirror`
- `git.repair_mirror`

Web Git mirror commands 当前只能打开 CLI-only notice 或 read-only repair review；不得直接升级为 Git writer。

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
- Git mirror queued / out-of-sync / CLI-only repair notice

状态来源必须是 runtime typed state 或 structured error code，不得由 view 层猜测。

## 9. Forbidden Patterns

- 把 `CommitInput` 嵌入 `Changes` group。
- 单 repo 默认展开 repositories list 并压过 commit flow。
- 默认展开 graph/history 并压过 staged/changes flow。
- 用 `git.*` command id 表示 Deve stage / commit authority。
- 让 view 组件直接改写 pending/staged side tables。
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

- request changes / staged state
- stage / unstage / discard / commit
- history / commit diff
- structured notices

View layer 不得直接操作 repo state、ledger state、pending/staged side tables、Git mirror state 或 `.notegit`。

## 11. Refactor Target

本章不新增 authority runtime。Source Control UI 应继续归属：

- `ui_shell`
- `application_control`
- `feature_runtime`
- `source_control_runtime`
- `diff_session_runtime`

若未来新增 `source_control_view_runtime`，必须先登记到 `docs/registry/runtime-skeleton-registry.md`，并说明它只拥有 view-local state。

## 本章相关命令

- `Source Control: Refresh`
- `Source Control: Stage All`
- `Source Control: Commit`
- `Source Control: Open History`
- `Source Control: Open Graph`

## 本章相关配置

- `source_control.show_repositories`
- `source_control.show_graph`
- `source_control.show_history`

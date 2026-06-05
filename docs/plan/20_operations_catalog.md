# 20_operations_catalog.md - Operations Catalog (操作目录)

## Metadata

- `Layer`: `Governance Contracts (non-layer ownership-axis slice)`
- `Status`: `Current MUST`
- `Version`: `0.0.1`
- `Last Review`: `2026-06-04`
- `Authority Owns`: `operation-flow catalog (Flow ID 键；atomic OpId 见 01_terminology §2.ter) / Extension Point Index / Replacement Point Index / Configuration Entry Index`
- `Authority Defers To`: `01_terminology, 03_storage, 06_backup, 07_network, 08_auth, 13_i18n (failure family codes), 15_settings (具体配置项定义), 各章末尾「本章相关配置」段`
- `Counterpart Feature`: `docs/features/operation-coverage.md`
- `Counterpart Acceptance`: `docs/acceptance-cases/00_index.md`
- `Primary Code Areas`: `crates/core/src/` 中所有 user operation 入口

## 1. Scope & Authority {#opid-catalog}

本章是**操作目录唯一权威**（plan B3.1 称 *OpId catalog*）：登记系统全部 operation-flow 及其治理属性。

- **目录键**：每行是一个 operation-flow，键为 operation-flow ID `flow.<domain>.<flow>`（与 `00_schema` §4 / `operation-coverage` 的 `Flow ID` 同名）。
- **与 OpId 的关系**：单个 flow 内的原子 **OpId**（`op.<domain>.<flow>.<verb>`，定义见 `01_terminology` §2.ter）由 operation 投影文件枚举，本表不逐条展开；本表每行属性即该 flow 全部原子 OpId 的治理归属。
- **Owns**：operation-flow 目录（§3）、Extension Point Index（§4）、Replacement Point Index（§5）、Configuration Entry Index（§6）。这些只是**索引**。
- **Defers To**：错误码定义归 `13_i18n#i18n-error-code-catalog`；配置项定义/默认值/环境变量名归各原章节与其「本章相关配置」段；runtime 状态全集归各章 §Runtime Boundary。本章只登记映射，不重定义语义。
- **Bijection**：本表 Flow ID 集合与 `docs/features/operation-coverage.md` 的 flow 集合 **MUST** 严格 1:1（吸收 `00_schema` §7）。

## 2. Column Legend (列释义)

- **Layer**（主 canonical 层）：`UO` User Operation · `II` Instruction Interface · `FC` Flow Coordination · `ED` Execution Domain。
- **Auth**（Authority Touched）：`L` ledger · `PW` Projection Workspace · `PO` Pending Overlay · `FS` pending_fs_ops · `—` 不触达 authority（含纯配置/纯读/纯渲染）。`PW` 仅指物理 Projection Workspace（Markdown 文件）；内存 Tree State / doc-list projection 不计入本轴。`PO` 仅指 Web thin-client session 的 Pending Overlay；Source Control 的 pending/staged 归 `FS`（`pending_fs_ops` / staging side table）。
- **WG**（Writer Gate Required）：`Y` 需通过 writer gate 方可产生权威副作用 · `N`。
- **Failure Family**：对应 `13_i18n#i18n-error-code-catalog` 的错误码族前缀；`(all)` 指该 op 是错误码映射本身；`—` 表示无后端结构化错误码（纯客户端/构建期）。
- **Ext**（Extension Point）：`Y` 暴露给 `19_plugins` / host function · `N`。
- **Repl**（Replacement Point）：`Y` 允许经 feature flag 替换实现 · `N`。
- **Owning Boundary**：该 op 权威副作用落点的 runtime boundary（章节/子章），可与 surface 域不同。
- **Gate**：进入该 op 的关键前置条件（简记）。

## 3. Operation Catalog

| Flow ID (`flow.*`) | Layer | Auth | WG | Failure Family | Ext | Repl | Owning Boundary | Gate |
|---|---|---|---|---|---|---|---|---|
| `flow.ai.chat` | UO | — | N | — | Y | Y | `16_ai_agent` | ai-chat-enabled |
| `flow.ai.trusted-external-agent-boundary` | ED | — | N | `PLUGIN_*` | Y | Y | `16_ai_agent` | trusted-cli-opt-in |
| `flow.auth.login` | UO | — | N | `AUTH_*` | N | N | `08_auth` | valid-credentials |
| `flow.auth.session-unauthorized` | II | — | N | `AUTH_*` | N | N | `08_auth` | missing/expired-token |
| `flow.commands.surface-action-routing` | II | — | N | — | N | N | `14_commands` | command-registered |
| `flow.commands.surface-mode-routing` | II | — | N | — | N | N | `14_commands` | mode-registered |
| `flow.cli.control-commands` | II | — | N | — | N | N | `14_commands#cli-commands` | cli-runtime-ready |
| `flow.cli.empty-command-guidance` | II | — | N | — | N | N | `14_commands#cli-commands` | no-args |
| `flow.cli.export-inspect` | ED | L | N | — | N | N | `06_backup` | export-target |
| `flow.cli.help-surface` | II | — | N | — | N | N | `14_commands#cli-commands` | — |
| `flow.cli.parse-command` | II | — | N | — | N | N | `14_commands#cli-commands` | well-formed-input |
| `flow.cli.repair-admin` | ED | L+PW | Y | `STORAGE_*` | N | N | `03_storage/repair` | admin-repair-cmd |
| `flow.cli.runtime-handoff` | FC | — | N | — | N | N | `14_commands#cli-commands` | runtime-ready |
| `flow.cli.server-runtime` | ED | — | N | `SYNC_*` | N | N | `07_network` | bind-ok |
| `flow.cli.projection-workspace-indexing` | ED | PW | N | `STORAGE_*` | N | N | `03_storage/projection` | workspace-mounted |
| `flow.doc.edit-confirmed-op` | UO | L+PW+PO | Y | `SYNC_*` | N | N | `03_storage/authority` | writer-gate+scope_nonce |
| `flow.doc.pending-navigation-guard` | UO | PO | N | `SYNC_*` | N | N | `09_web_thin_client_ledger` | pending-nonempty |
| `flow.i18n.error-mapping` | II | — | N | (all) | N | N | `13_i18n` | error-code-present |
| `flow.i18n.hardcoded-audit` | ED | — | N | — | N | N | `13_i18n` | audit-run |
| `flow.i18n.locale-error` | II | — | N | (all) | N | N | `13_i18n` | locale-load-fail |
| `flow.i18n.locale-surface-switch` | UO | — | N | — | N | N | `13_i18n` | locale-available |
| `flow.i18n.locale-selection` | UO | — | N | — | N | N | `13_i18n` | locale-available |
| `flow.i18n.localized-formatting` | ED | — | N | — | N | N | `13_i18n` | locale-active |
| `flow.net.key-exchange` | FC | — | N | `SYNC_*` | N | N | `07_network` | handshake-stage |
| `flow.net.sync-handshake` | FC | — | N | `SYNC_*` | N | N | `07_network` | connected |
| `flow.net.sync-transfer` | FC | L | N | `SYNC_*` | N | N | `07_network` | scope-bound |
| `flow.plugin.runtime-boundary` | ED | — | N | `PLUGIN_*` | Y | Y | `19_plugins` | capability-granted |
| `flow.release.ci` | ED | — | N | — | N | N | `18_release` | ci-trigger |
| `flow.release.tag-dispatch` | ED | — | N | — | N | N | `18_release` | tag-push |
| `flow.release.quality-gates` | ED | — | N | — | N | N | `18_release` | ci-stage |
| `flow.release.artifact-publish` | ED | — | N | — | N | N | `18_release` | gates-passed |
| `flow.release.delivery-verification` | ED | — | N | — | N | N | `18_release` | artifact-published |
| `flow.rendering.cursor-reveal` | UO | — | N | — | N | N | `10_rendering` | doc-open |
| `flow.rendering.checkbox-writeback` | UO | L+PW+PO | Y | `SYNC_*` | N | N | `03_storage/authority` | writer-gate |
| `flow.rendering.inline-source-reveal` | UO | — | N | — | N | N | `10_rendering` | doc-open |
| `flow.rendering.link-activation-gate` | UO | — | N | — | N | N | `10_rendering` | link-clicked |
| `flow.rendering.large-doc-prefetch` | FC | — | N | — | N | N | `10_rendering` | large-doc |
| `flow.rendering.large-doc-search-gate` | FC | — | N | — | N | N | `10_rendering` | large-doc |
| `flow.rendering.math-mermaid` | ED | — | N | — | N | N | `10_rendering` | block-present |
| `flow.rendering.math-source-projection` | UO | — | N | — | N | N | `10_rendering` | cursor-in-block |
| `flow.rendering.mermaid-source-projection` | UO | — | N | — | N | N | `10_rendering` | cursor-in-block |
| `flow.rendering.outline-navigation` | UO | — | N | — | N | N | `10_rendering` | outline-built |
| `flow.rendering.projection-refresh` | FC | — | N | — | N | N | `10_rendering` | doc-changed |
| `flow.repo.branch-switch` | UO | — | N | `SC_*` | N | N | `04_repository` | switch_nonce>scope_nonce |
| `flow.repo.file-op-shell-routing` | II | — | N | `SC_*` | N | N | `04_repository` | repo-selected |
| `flow.repo.file-operations` | UO | L+PW | Y | `SC_*` | N | N | `04_repository` | writer-gate+repo-scope |
| `flow.repo.open-doc` | UO | PW | N | `DOC_*` | N | N | `04_repository` | repo-selected |
| `flow.repo.switch` | UO | — | N | `SC_*` | N | N | `04_repository` | repo-available |
| `flow.sc.commit` | UO | L | Y | `SC_*` | N | N | `05_diff_logic` | staged-nonempty+writer-gate |
| `flow.sc.commit-and-push` | FC | L | Y | `SC_*` | N | N | `05_diff_logic` | staged+connected |
| `flow.sc.discard-file` | UO | FS+PW | Y | `SC_*` | N | N | `05_diff_logic` | file-pending |
| `flow.sc.discard-pending` | UO | — | Y | `SC_*` | N | N | `05_diff_logic` | pending-present |
| `flow.sc.history-commit-diff` | UO | — | N | `SC_*` | N | N | `05_diff_logic` | commit-exists |
| `flow.sc.merge-peer` | FC | L | Y | `SC_*` | N | N | `05_diff_logic` | peer-branch-available |
| `flow.sc.merge-runtime` | FC | L | Y | `SYNC_*` | N | N | `05_diff_logic` | conflict-resolved |
| `flow.sc.resolve-conflict` | UO | FS+PW | Y | `SC_*` | N | N | `05_diff_logic` | conflict-present |
| `flow.sc.stage-unstage` | UO | FS | Y | `SC_*` | N | N | `05_diff_logic` | change-present |
| `flow.search.query` | UO | — | N | — | N | N | `10_rendering` | index-ready |
| `flow.settings.env-defaults` | ED | — | N | — | N | N | `15_settings` | startup |
| `flow.settings.feedback-render` | UO | — | N | — | N | N | `15_settings` | — |
| `flow.settings.file-config` | ED | — | N | — | N | N | `15_settings` | config-file-present |
| `flow.settings.persistence-apply` | FC | — | N | — | N | N | `15_settings` | valid-setting |
| `flow.settings.surface-open` | UO | — | N | — | N | N | `15_settings` | — |
| `flow.settings.runtime-feedback` | II | — | N | — | N | N | `15_settings` | — |
| `flow.settings.ui-preferences` | UO | — | N | — | N | N | `15_settings` | — |
| `flow.settings.update` | UO | — | N | — | N | N | `15_settings` | valid-setting |
| `flow.settings.value-mutation` | UO | — | N | — | N | N | `15_settings` | valid-setting |
| `flow.tech-stack.dependency-policy` | ED | — | N | — | N | N | `17_tech_stack` | — |
| `flow.tech-stack.platform-release-channel` | ED | — | N | — | N | N | `17_tech_stack` | — |
| `flow.tech-stack.runtime-budget` | ED | — | N | — | N | N | `17_tech_stack` | defers→21_perf_budget |
| `flow.tech-stack.runtime-budget-check` | ED | — | N | — | N | N | `17_tech_stack` | ci-stage→21_perf_budget |
| `flow.ui.command-palette` | UO | — | N | — | N | N | `14_commands#command-palette-shortcuts` | palette-open |
| `flow.ui.context-action-routing` | II | — | N | — | N | N | `11_ui_design/index#context-action-surface` | action-registered |

## 4. Extension Point Index {#extension-point-index}

暴露给 `19_plugins` / host function 的扩展点（§3 Ext=Y）：

- `flow.ai.chat`、`flow.ai.trusted-external-agent-boundary` → 经 `16_ai_agent` 的 Trusted CLI Agent / native AI host 边界暴露。
- `flow.plugin.runtime-boundary` → capability-gated host function。

具体 host function 名与 capability 列表归 `19_plugins`；本节只做索引。

## 5. Replacement Point Index {#replacement-point-index}

允许经 feature flag 替换实现的替换点（§3 Repl=Y）：

- `flow.ai.chat`、`flow.ai.trusted-external-agent-boundary`（AI 能力整体可关闭/替换，见 `16_ai_agent` opt-in 边界）。
- `flow.plugin.runtime-boundary`（plugin runtime 可禁用/替换，见 `19_plugins`）。

feature flag 名与默认值归各原章节与 `17_tech_stack` feature matrix；本节只做索引。

## 6. Configuration Entry Index {#configuration-entry-index}

配置入口主索引。具体配置项定义、默认值与环境变量名 **Defers To** 各原章节：

| 配置域 | 权威章节 |
|---|---|
| 全局设置 / UI 偏好 / 持久化 | `15_settings` |
| 认证 / session / TLS | `08_auth` |
| 网络 / relay / 协议 | `07_network` |
| 备份 / 导出 locator | `06_backup` |
| locale / 错误码文案 | `13_i18n` |
| 技术栈 profile / feature matrix | `17_tech_stack` |

本节只登记「在哪一章定义配置」，不复制配置项本身（吸收 §7 单一可信来源）。

## 7. Projection Contract (投影合同)

- `docs/features/operation-coverage.md` 与 `docs/features/operations/*.md` 是本章的**投影**：它们绑定 flow → acceptance、枚举原子 `op.*.verb`，但 **MUST NOT** 新增本表未登记的 Flow ID，也不得定义本表以外的 operation-flow。
- 本表 Flow ID 集合与 operation-coverage flow 集合 **MUST** 严格 1:1。
- 新增 user operation 时：先在本表登记 Flow ID 与九列属性，再在 operations 投影文件补 flow 与原子 op；顺序不得颠倒。

## 8. Operational Action Matrix

本节登记 repo rename / projection repair 的运维动作，不新增 operation-flow ID；若这些动作暴露为产品级 UI/CLI flow，必须先在 §3 登记新的 `flow.*` 并同步更新 `docs/features/operation-coverage.md`。

| Action | Owning Boundary | Preconditions | Operator-visible Result |
|---|---|---|---|
| repo rename preflight | `04_repository#repo-health-and-repair` | `RepoId` 已解析；`expected_name_epoch` 匹配；目标 `<safe_repo_name>--<repo_id>` 可用；无 pending/staged/dirty/projection fault | 返回 rename plan 或结构化 reject |
| repo rename realign | `03_storage/projection#projection-locator-contract` | rename fact 已提交；watcher 已停止；`.notegit` identity marker 匹配同一 `RepoId` | workspace root 从旧 segment 移到新 segment，locator/catalog hint 更新 |
| projection degraded inspect | `22_reliability_observability#observation-to-health-mapping` | repo 处于 `DegradedProjection` 或 `DegradedLocator` | 输出 `repo_id`、当前 `RepoNameBinding`、workspace root、fault kind、可执行 repair action |
| projection rebuild / rematerialize | `03_storage/projection#projection-contract` | repo 未处于 dirty/staged/pending gate；ledger authority 可读 | 从 ledger 重建 projection/workspace，成功后回到 `Healthy` |
| failed realign recovery | `04_repository#repo-health-and-repair` | rename fact 已提交但 workspace realign 未完成 | 以 `RepoId` 为锚点重试 realign；不得用旧 repo name 或 URL 重新绑定身份 |

运维输出的所有 repo 字段都必须同时包含 `repo_id` 与当前 `repo_name`；机器可判定字段以 `repo_id` 为准，`repo_name` 只供人工识别。

## 9. Related Configuration (本章相关配置)

- 本章自身无独立配置项；配置入口索引见 §6，权威定义归各原章节。

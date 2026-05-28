# Coverage Matrix

本文件维护三层文档的稳定映射：

- `docs/plan/` = engineering blueprint
- `docs/features/` = manual feature verification via Chrome MCP
- `docs/acceptance-cases/` = automation-oriented validation

> Blueprint 列使用 `docs/plan/` 重排后的章节编号（A Foundation / B Authority Core /
> C Runtime Protocols / D Application+UI Shell / E Peripheral 各层内连续）。Feature Spec
> 与 Acceptance Cases 列保持各自目录的独立编号，不随 plan 重排改变。

## Chapter Mapping

| Blueprint | Feature Spec | Acceptance Cases | Primary Verification |
| --- | --- | --- | --- |
| `01_terminology` | `01_terminology` | `01_terminology` | glossary / lint / naming checks |
| `02_positioning` | `02_positioning` | `02_positioning` | CLI + architecture boundary checks |
| `03_storage/index` | `04_storage` | `07_storage_repo` | storage/runtime/repair automation（index + authority/projection/watcher/repair 子文件） |
| `04_repository` | `06_repository` | `07_storage_repo` | repo/scope automation + Chrome MCP |
| `05_diff_logic` | `07_diff_logic` | `04_diff` | diff/merge/source-control automation |
| `06_backup` | `06_repository` | `07_storage_repo` | backup/restore locator boundary automation |
| `07_network` | `05_network` | `06_network` | protocol + reconnect automation |
| `08_auth` | `09_auth` | `08_auth` | auth/security automation |
| `09_web_thin_client_ledger` | `16_web_thin_client_ledger` | `06_network`, `07_storage_repo` | pending/ack/reject automation + Chrome MCP |
| `10_rendering` | `03_rendering` | `03_rendering` | Chrome MCP + rendering automation |
| `11_ui_design/index` | `08_ui_design` | `05_ui`, `13_ui_mobile_chat_regression` | Chrome MCP + UI automation |
| `11_ui_design/01_web` | `08_ui_design_01_web` | `05_ui` | Web shell automation + Chrome MCP |
| `11_ui_design/02_desktop` | `08_ui_design_02_desktop` | `05_ui` | desktop shell automation |
| `11_ui_design/03_mobile` | `08_ui_design_03_mobile` | `05_ui`, `13_ui_mobile_chat_regression` | mobile shell automation + Chrome MCP |
| `12_source_control_ui` | `07_diff_logic`, `08_ui_design_02_desktop` | `04_diff`, `05_ui` | Source Control UI reference + automation |
| `13_i18n` | `11_i18n` | `09_i18n` | locale/error-code automation |
| `14_commands` | `12_commands` | `11_commands_settings` | command/control automation |
| `15_settings` | `13_settings` | `11_commands_settings` | settings persistence automation |
| `16_ai_agent` | `10_ai_agent` | `10_plugins` | external runtime boundary automation |
| `17_tech_stack` | `14_tech_stack` | `12_tech_release` | build/runtime budget checks |
| `18_release` | `15_release` | `12_tech_release` | packaging/CI/release automation |
| `19_plugins` | `17_plugins` | `10_plugins` | plugin/runtime boundary automation |

### Governance Contracts (20-23)

20-23 是 **Governance Contracts**（与 A-E 模块层正交的 ownership-axis 切片，非产品 feature 章），
不进入上面的 A-E 三层映射表。它们的对照与验证沿治理轴表达：feature 侧统一投影到
`operation-coverage.md`，验证以 `scripts/plan-coverage.sh` 的治理子检查为主，而非 Chrome MCP walkthrough。

| Blueprint | Counterpart Feature | Counterpart Acceptance | Primary Verification |
| --- | --- | --- | --- |
| `20_operations_catalog` | `operation-coverage.md`（章 20 的投影） | `14_operation_flow_refs` / `00_index`（op-flow ↔ case 绑定） | operation bijection（`check-architecture-registry.sh`）+ `--check-reverse-coverage` |
| `21_perf_budget` | `operation-coverage.md`（perf-sensitive flows） | `12_tech_release`（`PERF-001`） | `plan-coverage.sh --check-perf-budget` |
| `22_reliability_observability` | `operation-coverage.md`（release / observability flows） | `12_tech_release`（`REL-002`） | `--check-reverse-coverage` + `--check-metadata-completeness` |
| `23_threat_model` | `operation-coverage.md`（auth / security flows） | `10_plugins`（`PLUG-001`）、`08_auth`（`AUTH-*`） | `--check-no-adr-plan-ref` + auth/security automation |

> 上述治理 acceptance 用例（`PERF-001` / `REL-002` / `PLUG-001` / `AUTH-*`）均已在所列
> `docs/acceptance-cases/` 文件中定义并绑定（automated / walkthrough），由
> `scripts/check-acceptance-bindings.sh` 校验（0 unbound）。

### Non-Matrix Documents

The following documents exist under `docs/` but do not participate in the three-layer matrix:

| Document | Type | Notes |
| --- | --- | --- |
| `docs/plan/00_engineering_constitution.md` | Governing Rule | Cross-chapter skeleton governance; not a feature chapter |
| `docs/tasks/18_infra_runtime.md` | Implementation Blueprint | Infra-first module boundaries; guided by but does not override A/B/C layer chapters |
| `docs/tasks/19_repo_refactor_blueprint.md` | Implementation Blueprint | Repo restructuring migration order |
| `docs/overview/architecture.md` | Architecture Overview | Cross-layer 4-layer cascade map, human entry point |
| `docs/overview/architecture-doc.lisp` | Architecture View | Doc-derived view; references plan anchors |
| `docs/overview/architecture-code.lisp` | Architecture View | Code-derived view; references source tree |
| `docs/overview/architecture-diff.md` | Verification Report | Divergence between doc and code views |
| `docs/registry/runtime-skeleton-registry.md` | Controlled Registry | Runtime name/status/current module path registry; referenced from the plan index |
| `docs/plan/plugins/agent_bridge/01_agent_bridge.md` | Design Note | Dual-channel AI architecture; referenced from `16_ai_agent.md` Metadata |
| `docs/ai-chat-streaming.md` | Design Note | Streaming bridge design; referenced from `16_ai_agent.md` Metadata |

## Rules

- Every `docs/plan/` chapter (01-19 + 11_ui sub-chapters) must have a corresponding row in the Chapter Mapping table; 20-23 Governance Contracts are mapped separately in the Governance Contracts table (ownership-axis slice, not A-E feature chapters).
- Every `docs/features/` chapter must define at least one Chrome MCP walkthrough, **except** `01_terminology` and `02_positioning` which use `Verification: glossary-only / boundary-only`.
- Every `docs/acceptance-cases/` file must map to at least one stable automation surface.
- Non-automated acceptance cases must be listed in `docs/acceptance-bindings.tsv`
  with a binding type and evidence document; `scripts/plan-coverage.sh` validates
  the case id, binding type, and evidence path.
- A single acceptance file may cover multiple blueprint chapters, but the mapping must be explicit here.
- `11_ui_design/index.md` remains the shared cross-surface feature chapter.
- `11_ui_design/01_web`, `/02_desktop`, `/03_mobile` define Web / Desktop / Mobile shell behavior at the feature level and mirror the adapter split in `docs/plan/`.

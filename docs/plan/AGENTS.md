<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

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
| `验收清单.md` | Acceptance checklist (Chinese) |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `acceptance-cases/` | Detailed acceptance test scenarios |
| `plugins/` | Plugin system design documents |

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
//!   - 04_storage.md §Watcher Architecture
//!   - 04_storage.md §Inode/DocId Mapping & Watcher Service
```

**规则**：
- 注解格式为 `//! plan_ref:` 紧接 YAML-ish 列表；每行一条，格式 `  - <chapter_file> §<section>`。
- 纯工具/util 模块（如 `utils/path.rs`）可使用 `//! plan_ref: infra` 标记为基础设施，豁免章节追溯。
- 同一模块 MAY 引用多个章节；跨域模块应优先拆分而非堆叠引用。
- 删除代码前 MUST 核对其 `plan_ref` 对应条款是否已从 plan 中移除或重新分配。

### Layer 2 — CI Coverage Check (覆盖率扫描)

`scripts/plan-coverage.sh` 扫描 `crates/` 与 `apps/` 下所有 `.rs` 文件，输出：
1. 无 `plan_ref` 注解的模块清单（warning，非阻塞）
2. 引用了已不存在的章节或章节名的模块清单（error，阻塞）
3. plan 章节的反向覆盖矩阵：每个 `§section` 被哪些代码文件引用

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
- 单文件行数检查：`crates/` 与 `apps/` 下 `.rs` 文件超过 250 行即阻塞（熔断阈值）
- i18n facade 检查：`apps/web/src/components/` 下硬编码中/英文用户可见字符串即阻塞

以上检查统一封装于 `scripts/plan-coverage.sh`，单入口执行全部验证。

<!-- MANUAL: -->

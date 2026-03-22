<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# deve-note plan

## Purpose

Comprehensive design document for Deve-Notebook, organized into 16 chapters covering every aspect of the system — from terminology and storage architecture to UI design, networking, authentication, and release strategy. This is the primary reference for all implementation work.

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
| `10_i18n.md` | Internationalization strategy |
| `11_plugins.md` | Plugin boundary, ledger-aware note host API |
| `11b_ai_integration.md` | AI chat integration and tool-use protocol |
| `12_commands.md` | Command palette and keyboard shortcuts |
| `13_settings.md` | Settings system and persistence |
| `14_tech_stack.md` | Technology choices and rationale |
| `15_release.md` | Build, packaging, and deployment |
| `16_web_thin_client_ledger.md` | Web thin client, repo-scoped state machine, scope gates |
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
- Do not modify plan files unless asked — they are reference documents.

<!-- MANUAL: -->

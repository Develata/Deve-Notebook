<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# source_control

## Purpose

Source control sidebar panel. Shows changes, staged/unstaged sections, commit interface, and history.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Source control panel component |
| `changes.rs` | Changes overview |
| `change_item.rs` | Individual change item |
| `staged_section.rs` | Staged changes section |
| `unstaged_section.rs` | Unstaged changes section |
| `commit.rs` | Commit interface |
| `commit_ai.rs` | Commit message AI generation runtime |
| `commit_ai_runtime.rs` | Commit AI backend/effect planning |
| `history.rs` | Commit history view |
| `graph_panel.rs` | Read-only source-control graph projection panel |
| `context_menu.rs` | Right-click context menu |
| `repositories.rs` | Multi-repo listing |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `commit_ai/` | Commit AI backend, runtime, and signal tests |
| `graph_panel/` | Graph panel state and copy unit tests |
| `history/` | History compare reset/state helpers |
| `history_compare_banner/` | History compare banner copy helpers |

## For AI Agents

### Working In This Directory

- Remote branches are read-only; buttons that mutate Source Control state must visually and behaviorally respect that gate.
- Header and section menus should be real button-driven menus with outside-click dismissal and automatic close after selection.
- `repositories.rs` should reuse the shared sidebar repo switcher semantics rather than inventing a second repo-switch flow.
- History compare reset logic should only clear meaningful compare state; collapsed renders must not continuously wipe selection.

<!-- MANUAL: -->

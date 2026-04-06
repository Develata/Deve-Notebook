<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# sidebar

## Purpose

File explorer sidebar with tree view, repo switcher, and source control panel integration.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Sidebar component |
| `explorer.rs` | File explorer tree |
| `tree.rs` | Tree view rendering |
| `item.rs` | Tree item component |
| `types.rs` | Sidebar type definitions |
| `components.rs` | Shared sidebar components |
| `path_utils.rs` | Path display utilities |
| `repo_switcher.rs` | Repository switching UI |
| `extensions.rs` | Extension panel |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `source_control/` | Source control panel |

## For AI Agents

### Working In This Directory

- `repo_switcher.rs` is the canonical repo switch UI for both Explorer and Source Control entry points.
- Repo switcher trigger and menu items should stay keyboard/click accessible `button` elements, with outside-click dismissal.
- Explorer-side actions that open new windows must preserve existing query params when appending `doc=...`.

<!-- MANUAL: -->

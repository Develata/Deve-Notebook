<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# diff_view

## Purpose

Source control diff viewer with split and unified modes. Implements patience diff algorithm, line-level rendering, fold/expand, viewport-based lazy rendering, and hunk navigation.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | DiffView component entry |
| `state/` | Diff view state management, compute scheduling, and tests |
| `body.rs` | Diff body rendering |
| `header.rs` | Diff header (file path, stats) |
| `line_render.rs` | Line rendering with word-level highlights |
| `split_pane.rs` | Split view (side-by-side) |
| `unified.rs` | Unified diff view |
| `fold.rs` | Fold/expand unchanged regions |
| `navigation.rs` | Hunk navigation |
| `viewport.rs` | Viewport-based lazy rendering |
| `cache.rs` | Diff computation cache |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `model/` | Diff data model and algorithms |

## For AI Agents

### Working In This Directory

- See `07_diff_logic.md` in deve-note plan for diff design.
- Uses patience diff algorithm, falls back to Myers.

<!-- MANUAL: -->

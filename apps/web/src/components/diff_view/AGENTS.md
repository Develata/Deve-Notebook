<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# diff_view

## Purpose

Thin source-control diff viewer for Core-provided typed projections. It renders split/unified rows, backend-provided folds and hunks, viewport virtualization, and merge intents without computing diff semantics in Web.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | DiffView session, edit debounce, status, and header |
| `projection.rs` | Projection controls, hunk navigation, and virtual viewport |
| `projection_model.rs` | Immutable row/fold selection and projection validation |
| `projection_row.rs` | Split/unified DOM row rendering |
| `projection_text.rs` | UTF-16 wire ranges to UTF-8 DOM text adapter |
| `surface.rs` | Loading, unavailable, retry, and projection surface state |
| `conflict_actions.rs` | Typed merge-resolution intents |

## For AI Agents

### Working In This Directory

- See `docs/plan/05_diff_logic.md` and `docs/plan/10_rendering.md`.
- Diff algorithms, hunk/fold decisions, and word ranges belong to Core; this directory must remain projection-only.

<!-- MANUAL: -->

<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# drawers

## Purpose

Swipeable left (file explorer) and right (outline/chat) drawers for mobile layout.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Drawer module declarations |
| `left.rs` | Left drawer (file explorer) |
| `right.rs` | Right drawer (outline/chat) |

## For AI Agents

### Working In This Directory

- Left drawer tab entries and `More` menu items must be real buttons; clicking a row should switch the view, not silently toggle pin state.
- Successful doc navigation or sidebar tab selection should close the mobile drawer immediately.

<!-- MANUAL: -->

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
| `left_more_menu.rs` | Mobile left drawer More menu and pin-state policy |
| `left_more_menu_test.rs` | Mobile More menu marker, close, and pin-toggle unit tests |
| `right.rs` | Right drawer (outline/chat) |

## For AI Agents

### Working In This Directory

- Left drawer tab entries and `More` menu items must be real buttons; clicking a row should switch the view, not silently toggle pin state.
- Successful doc navigation or sidebar tab selection should close the mobile drawer immediately.

<!-- MANUAL: -->

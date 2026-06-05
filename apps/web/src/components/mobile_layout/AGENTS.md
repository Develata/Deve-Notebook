<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# mobile_layout

## Purpose

Mobile-responsive layout with touch gestures, swipeable drawers, and compact toolbar.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | MobileLayout component |
| `header.rs` | Mobile header |
| `footer.rs` | Mobile footer |
| `content.rs` | Main content area |
| `surface_switcher.rs` | Mobile current surface capsule and bottom-sheet tab switcher |
| `toolbar.rs` | Mobile toolbar |
| `gesture.rs` | Touch gesture handling |
| `effects.rs` | Layout effects and breakpoints |
| `chat_sheet.rs` | Bottom sheet for chat |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `drawers/` | Left and right swipeable drawers |

## For AI Agents

### Working In This Directory

- See `08_ui_design_03_mobile.md` in deve-note plan.
- Edge swipe gestures must not steal taps from interactive controls near the screen edge.
- Mobile header actions are intentionally limited to a single `Home / Open Index / Command Palette` set.
- Drawer state should close explicitly on successful navigation or tab switches rather than relying on stale a11y snapshots.

<!-- MANUAL: -->

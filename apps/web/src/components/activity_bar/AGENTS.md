<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# activity_bar

## Purpose

VS Code-style vertical activity bar for switching between sidebar views.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | ActivityBar component |
| `popup_menu.rs` | Popup menu |
| `types.rs` | Activity bar item types |

## For AI Agents

### Working In This Directory

- In the `More...` popup, the menu row selects a view; pin/unpin is a separate action and must not hijack row clicks.
- View selection should close the popup immediately after activation.

<!-- MANUAL: -->

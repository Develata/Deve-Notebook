<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# components

## Purpose

All UI components organized by feature area. Each subdirectory is a self-contained feature module.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | Component module declarations and re-exports |
| `main_layout.rs` | Main application layout facade |
| `main_layout_runtime.rs` | Main layout runtime shell |
| `desktop_layout.rs` | Desktop layout facade with sidebar, editor, panels |
| `desktop_chat_panel.rs` | Desktop AI chat panel |
| `header.rs` | Top header bar |
| `bottom_bar.rs` | Bottom status bar |
| `layout_context.rs` | Layout state context (sidebar open, panel sizes) |
| `dropdown.rs` | Reusable dropdown component |
| `dropdown/` | Dropdown placement helpers |
| `outline.rs` | Document outline panel |
| `settings.rs` | Settings dialog |
| `settings_sections_policy.rs` | Settings section button-state and disabled-state policy helpers |
| `settings_sections_policy/` | Settings policy helper unit tests |
| `merge_modal.rs` | Merge conflict resolution modal |
| `merge_panel.rs` | Merge panel for side-by-side conflict view |
| `disconnect_overlay.rs` | Connection lost overlay |
| `spectator_overlay.rs` | Read-only spectator mode overlay |
| `playback.rs` | Document history playback component |
| `sidebar_menu/` | Sidebar context menu item renderer |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `activity_bar/` | VS Code-style activity bar |
| `branch_switcher/` | Branch switching UI |
| `chat/` | AI chat interface |
| `command_palette/` | Command palette (Ctrl/Cmd+Shift+P) |
| `dashboard/` | System dashboard |
| `diff_view/` | Source control diff viewer |
| `desktop_layout/` | Desktop layout banner, content, handles, and sidebar helpers |
| `icons/` | SVG icon components |
| `login/` | Login page and auth state |
| `main_layout/` | Main layout callbacks, context providers, and setup helpers |
| `main_layout_runtime/` | Main layout body and overlay helpers |
| `mobile_layout/` | Mobile-responsive layout with drawers |
| `outline_render/` | Markdown outline and KaTeX rendering |
| `search_box/` | Search with file operations |
| `sidebar/` | File explorer and source control panel |

## For AI Agents

### Working In This Directory

- Components use Leptos `#[component]` macro with `view!{}` templates.
- State comes from `hooks/use_core/` via `provide_context` / `use_context`.
- See `08_ui_design*.md` in deve-note plan for UI design specs.

<!-- MANUAL: -->

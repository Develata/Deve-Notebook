# Desktop Web Shell Browser Smoke

Date: 2026-05-13

## Scope

- Plan source: `docs/plan/08_ui_design_02_desktop.md`
- Acceptance binding: `docs/acceptance-cases/05_ui.md` `UI-DESK-001..003`
- Runtime surface: Web shell at desktop width (`window.innerWidth = 1280`)
- Data root: isolated temp root `/tmp/deve-desktop-smoke-20260513-CZkn9K`
- Server: `cargo run -p deve_cli --bin deve_cli -- serve --dev --port 32117`
- Auth: development defaults, already logged in as `admin/admin`

## Automated Guards

- `bash scripts/check-ui-desktop-baseline.sh` -> passed
- `cargo test -p deve_web desktop_diff_scroll -- --nocapture` -> 4 passed
- `cargo test -p deve_web desktop_layout_resize -- --nocapture` -> 3 passed
- `cargo test -p deve_web unified_search_mode -- --nocapture` -> 3 passed

## Chrome MCP Smoke

### Desktop Layout

- Desktop breakpoint was active at `1280x900`.
- `data-deve-desktop-col="1-sidebar"` rendered at 250px initially.
- `data-deve-desktop-col="3-editor"` and `data-deve-desktop-col="4-outline"` rendered after opening `desktop-smoke.md`.
- `data-deve-desktop-col="5-chat"` rendered at 350px initially.
- No disconnected overlay was visible after load or reload.

### Resize Persistence

- Sidebar resize handle drag changed width from `250` to `300`.
- Right panel resize handle drag changed width from `350` to `390`.
- `localStorage.ui_sidebar_width` persisted as `300`.
- `localStorage.ui_right_panel_width` persisted as `390`.
- Reload preserved sidebar width `300` and right panel width `390`.

### Diff Scroll Sync

- External workspace edit created one Source Control modified entry for `desktop-smoke.md`.
- Opening the change rendered `data-deve-desktop-col="2-diff-old"` and `data-deve-desktop-col="3-editor"`.
- Large diff body produced scrollable old/new panes.
- Programmatic scroll on Col3 to ~60% updated Col2 to the same ratio.
- Old pane max scroll: `2866`.
- New pane max scroll: `2866`.
- Old ratio after sync: `0.6001395673`.
- New ratio after sync: `0.6001395673`.

### Unified Search Mode Routing

- Command Palette opened with `data-deve-search-mode="command"`.
- Query `>toggle` kept mode `command`.
- Query `@branch` switched mode to `branch`.
- Query `desktop-smoke.md` switched mode to `file`.

### Browser Health

- Current navigation console `error` / `warn` list was empty.
- Current navigation network requests for document/fetch endpoints returned 200.
- Preserved console history contained stale errors from an older stopped smoke server on port `32026`; those entries were not from the current `32117` navigation.

## Result

`UI-DESK-001..003` are browser-smoke verified for the current Web shell. No code changes were required.

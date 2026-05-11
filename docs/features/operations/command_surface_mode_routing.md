# command_surface_mode_routing.md - 命令入口模式路由链

## Metadata

- `Flow ID`: `flow.commands.surface-mode-routing`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`, `docs/features/06_repository.md`
- `Related Acceptance Cases`: `CMD-002`, `CMD-003`, `CMD-004`, `UI-DESK-003`

## Operations

### `op.commands.surface.open-unified-search`

- `Name`: `Open Unified Command Surface`
- `Surface`: `keyboard-shortcut-or-overlay`
- `Trigger`: open command palette, quick open, or unified search surface
- `Preconditions`: workspace shell is loaded
- `Immediate Result`: a shared query input becomes active
- `Application Entry`: `apps/web/src/components/command_palette/`, `apps/web/src/components/search_box/`

### `op.commands.surface.type-mode-prefix`

- `Name`: `Type Command Mode Prefix`
- `Surface`: `search-input`
- `Trigger`: type `>`, `@`, `+`, or a plain file query
- `Preconditions`: shared command surface is visible
- `Immediate Result`: query draft now carries routing intent instead of raw text only
- `Application Entry`: `apps/web/src/components/search_box/mod.rs`, `apps/web/src/components/search_box/logic/providers.rs`

### `op.commands.surface.observe-routed-provider`

- `Name`: `Observe Routed Provider Results`
- `Surface`: `search-results`
- `Trigger`: provider router evaluates the current query
- `Preconditions`: query draft is available
- `Immediate Result`: result list is sourced from command, branch, file, or file-op providers according to mode
- `Application Entry`: `apps/web/src/components/search_box/logic/providers.rs`, `apps/web/src/components/search_box/providers/command.rs`, `apps/web/src/components/search_box/providers/branch.rs`, `apps/web/src/components/search_box/providers/file.rs`

## Response Flow

1. User opens a shared command/search surface.
2. Instruction interface reads the current query shape, not just the literal text.
3. Flow coordination routes the query to command, branch, file, or file-operation providers.
4. Execution domains touched here are `commands` and `tree`, because the router selects provider families and repo-scoped candidates before deeper business execution begins.

## Notes

- This flow models provider selection, not the eventual business action after picking a result.
- The `>` / `@` / `+` prefixes are part of the routing contract, not cosmetic UI syntax.
- Main objects: `command::registry`, `search::mode`, `search::action`.

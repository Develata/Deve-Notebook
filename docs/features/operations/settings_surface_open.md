# settings_surface_open.md - Settings 入口打开链

## Metadata

- `Flow ID`: `flow.settings.surface-open`
- `Domain`: `settings`
- `Related Feature Chapters`: `docs/features/13_settings.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `SET-005`, `CMD-002`

## Operations

### `op.settings.surface.open-ui`

- `Name`: `Open Settings From UI`
- `Surface`: `workspace-shell`
- `Trigger`: click Settings entry from the visible shell
- `Preconditions`: application shell is loaded
- `Immediate Result`: settings modal or settings panel becomes visible
- `Application Entry`: `apps/web/src/components/settings.rs`

### `op.settings.surface.open-command`

- `Name`: `Open Settings From Command Surface`
- `Surface`: `command-surface`
- `Trigger`: invoke settings from command palette or command entry
- `Preconditions`: command surface is available
- `Immediate Result`: settings surface is routed without leaving the current app context
- `Application Entry`: `apps/web/src/components/command_palette/`, `apps/web/src/components/settings.rs`

### `op.settings.surface.view-groups`

- `Name`: `View Settings Groups`
- `Surface`: `settings-modal`
- `Trigger`: settings surface finishes opening
- `Preconditions`: settings surface loaded successfully
- `Immediate Result`: grouped settings sections are visible instead of a flat list
- `Application Entry`: `apps/web/src/components/settings_sections.rs`

## Response Flow

1. User opens settings from UI or command surface.
2. Instruction interface routes the request into the settings shell.
3. Flow coordination loads grouped settings sections.
4. Execution domains are settings UI shell and command/control.

## Notes

- Opening settings is distinct from mutating a setting value.
- Main objects: `settings::surface`, `ui::preference`, `command::route`.

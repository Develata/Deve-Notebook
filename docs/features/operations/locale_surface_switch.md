# locale_surface_switch.md - 语言切换共享入口链

## Metadata

- `Flow ID`: `flow.i18n.locale-surface-switch`
- `Domain`: `i18n`
- `Related Feature Chapters`: `docs/features/11_i18n.md`, `docs/features/12_commands.md`, `docs/features/13_settings.md`
- `Related Acceptance Cases`: `I18N-002`, `SET-005`, `CMD-002`

## Operations

### `op.locale.switch.invoke-command-toggle`

- `Name`: `Invoke Locale Toggle From Command Surface`
- `Surface`: `command-palette`
- `Trigger`: select the `lang` / toggle language command
- `Preconditions`: command palette is open and command registry is available
- `Immediate Result`: locale switch intent leaves the command surface
- `Application Entry`: `apps/web/src/components/command_palette/registry.rs`

### `op.locale.switch.select-settings-locale`

- `Name`: `Select Locale From Settings`
- `Surface`: `settings-modal`
- `Trigger`: click `English` or `中文`
- `Preconditions`: settings modal is open
- `Immediate Result`: locale switch intent leaves the settings surface
- `Application Entry`: `apps/web/src/components/settings.rs`

### `op.locale.switch.apply-locale-context`

- `Name`: `Apply Locale Context`
- `Surface`: `locale-runtime`
- `Trigger`: locale switch intent reaches the shared locale signal
- `Preconditions`: locale context is available in the current app shell
- `Immediate Result`: active `Locale` changes to the chosen target
- `Application Entry`: `apps/web/src/i18n/mod.rs`, `apps/web/src/components/settings.rs`, `apps/web/src/components/command_palette/registry.rs`

### `op.locale.switch.observe-rerender`

- `Name`: `Observe Localized Rerender`
- `Surface`: `workspace-shell`
- `Trigger`: locale context changes
- `Preconditions`: localized UI text reads through the i18n facade
- `Immediate Result`: labels, titles, and command strings re-render in the new locale
- `Application Entry`: `apps/web/src/i18n/`, `apps/web/src/components/`

## Response Flow

1. User triggers locale change from command palette or settings.
2. Instruction interface converts both surfaces into the same locale-switch intent.
3. Flow coordination applies the shared locale context and keeps surface-specific triggers out of the catalog/runtime logic.
4. Execution domains are `commands`, `settings`, and `i18n`.

## Notes

- This flow is the shared bridge between trigger surfaces and the locale authority path.
- It does not replace `settings_ui_preferences.md` or `i18n_locale_selection.md`; it explains the reusable cross-surface contract they both touch.
- Main objects: `command::registry`, `locale::selection`, `i18n::catalog`, `ui::preference`.

# settings_ui_preferences.md - Settings UI 偏好链

## Metadata

- `Flow ID`: `flow.settings.ui-preferences`
- `Domain`: `settings`
- `Related Feature Chapters`: `docs/features/13_settings.md`, `docs/features/11_i18n.md`
- `Related Acceptance Cases`: `SET-005`, `I18N-001`, `I18N-002`

## Operations

### `op.settings.ui.select-language`

- `Name`: `Select UI Language`
- `Surface`: `settings-modal`
- `Trigger`: click English or 中文 in Settings
- `Preconditions`: settings modal is open
- `Immediate Result`: locale signal updates and labels re-render
- `Application Entry`: `apps/web/src/components/settings.rs`

### `op.settings.ui.select-theme`

- `Name`: `Select Theme Preference`
- `Surface`: `settings-modal`
- `Trigger`: click Auto, Light, or Dark in Settings
- `Preconditions`: settings modal is open
- `Immediate Result`: browser-local theme preference updates and root theme marker re-renders
- `Application Entry`: `apps/web/src/components/settings_sections.rs`,
  `apps/web/src/components/settings_prefs.rs`

### `op.settings.ui.select-editor-preference`

- `Name`: `Select Editor Preference`
- `Surface`: `settings-modal`
- `Trigger`: click Word Wrap or Density controls, or submit the maximum document tab count input with blur / Enter / change
- `Preconditions`: settings modal is open
- `Immediate Result`: browser-local editor markers or committed maximum document tab count update without writing repo authority
- `Application Entry`: `apps/web/src/components/settings_sections.rs`,
  `apps/web/src/components/settings_prefs.rs`
- `Notes`: maximum document tab count is persisted as `deve.ui.max_document_tabs`; the input keeps a draft while typing so partial two-digit values do not trigger eviction. It only limits `DocumentTab` eviction in the UI shell and does not persist open tab history.

### `op.settings.ui.select-sync-mode`

- `Name`: `Select Sync Mode`
- `Surface`: `settings-modal`
- `Trigger`: click Auto or Manual sync mode
- `Preconditions`: core sync context is available
- `Immediate Result`: sync mode callback receives the selected mode
- `Application Entry`: `apps/web/src/components/settings_sections.rs`

### `op.settings.ui.select-ai-backend`

- `Name`: `Select AI Backend`
- `Surface`: `settings-modal`
- `Trigger`: click Native or Trusted CLI backend
- `Preconditions`: backend capability check has completed
- `Immediate Result`: AI mode updates or disabled state explains why it cannot
- `Application Entry`: `apps/web/src/components/settings_sections.rs`

### `op.settings.ui.toggle-ai-chat-panel`

- `Name`: `Toggle AI Chat Panel Visibility`
- `Surface`: `settings-modal`
- `Trigger`: click Show or Hide in the AI Chat panel visibility control
- `Preconditions`: settings modal is open and ChatControl is available
- `Immediate Result`: browser-local AI Chat visibility updates; hidden state removes the desktop Chat panel and its divider without changing AI backend config
- `Application Entry`: `apps/web/src/components/settings_sections.rs`,
  `apps/web/src/components/settings_prefs.rs`

## Response Flow

1. User changes a visible settings control.
2. Instruction interface is the settings modal callback.
3. Flow coordination updates local UI state or delegates to core callbacks.
4. Execution domains are settings, i18n, browser prefs, sync, layout shell, and plugin/agent capability.

## Notes

- UI preferences must not directly mutate ledger authority state.
- Settings 中的语言切换与 command palette 中的 `lang` 命令共用一条跨表面 locale 切换链，见 `locale_surface_switch.md`。
- Main objects: `ui::preference`, `locale::selection`, `editor::local-preference`, `editor::document-tab-limit`, `ai::backend-capability`, `ai::chat-panel-visibility`.

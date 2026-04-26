# settings_runtime_feedback.md - Settings 运行时反馈链

## Metadata

- `Flow ID`: `flow.settings.runtime-feedback`
- `Domain`: `settings`
- `Related Feature Chapters`: `docs/features/13_settings.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `SET-003`, `SET-006`, `CMD-002`, `AI-006`

## Operations

### `op.settings.feedback.observe-visual`

- `Name`: `Observe Visual Setting Feedback`
- `Surface`: `workspace-shell`
- `Trigger`: language, panel, or layout preference changes
- `Preconditions`: setting has visible UI effect
- `Immediate Result`: user sees changed labels, layout, or selected state
- `Application Entry`: `apps/web/src/components/settings.rs`

### `op.settings.feedback.observe-disabled`

- `Name`: `Observe Disabled Reserved Setting`
- `Surface`: `settings-modal`
- `Trigger`: view future or unavailable setting such as Trusted CLI
- `Preconditions`: capability gate is not satisfied
- `Immediate Result`: disabled control shows explicit reason instead of silently failing
- `Application Entry`: `apps/web/src/components/settings_sections.rs`

### `op.settings.feedback.inspect-runtime`

- `Name`: `Inspect Runtime Setting Feedback`
- `Surface`: `cli-or-log`
- `Trigger`: inspect startup logs, profile behavior, or low-spec runtime effects
- `Preconditions`: runtime has loaded effective config
- `Immediate Result`: operator can verify active profile and enabled services
- `Application Entry`: `apps/cli/src/main.rs`, `apps/cli/src/server/mod.rs`

## Response Flow

1. User observes the result of a setting rather than editing the source value.
2. Instruction interface is UI state rendering, disabled-state rendering, or runtime log output.
3. Flow coordination maps setting state to visible or inspectable feedback.
4. Execution domains are settings, i18n, protocol/server runtime, and tech-stack constraints.

## Notes

- Feedback operations prevent “changed but invisible” settings drift.
- Main objects: `settings::feedback`, `runtime::profile`, `ui::preference`.

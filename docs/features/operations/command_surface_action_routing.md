# command_surface_action_routing.md - 命令入口动作路由链

## Metadata

- `Flow ID`: `flow.commands.surface-action-routing`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`, `docs/features/06_repository.md`
- `Related Acceptance Cases`: `CMD-002`, `CMD-003`, `CMD-004`, `REPO-FEAT-01`, `REPO-FEAT-02`

## Operations

### `op.commands.surface.choose-result`

- `Name`: `Choose Routed Result`
- `Surface`: `keyboard-or-pointer`
- `Trigger`: press `Enter` or click the selected result
- `Preconditions`: routed result list is non-empty
- `Immediate Result`: one `SearchAction` becomes the chosen dispatch target
- `Application Entry`: `apps/web/src/components/search_box/logic/selection.rs`, `apps/web/src/components/search_box/ui_sections.rs`

### `op.commands.surface.dispatch-action`

- `Name`: `Dispatch SearchAction`
- `Surface`: `search-surface`
- `Trigger`: `execute_action` receives the selected result action
- `Preconditions`: selected result is not `SearchAction::Noop`
- `Immediate Result`: one of `RunCommand`, `OpenDoc`, `SwitchBranch`, `CreateDoc`, or `FileOp` is dispatched
- `Application Entry`: `apps/web/src/components/search_box/logic/execute.rs`, `apps/web/src/components/search_box/types.rs`

### `op.commands.surface.enter-target-flow`

- `Name`: `Enter Routed Target Flow`
- `Surface`: `workspace-shell`
- `Trigger`: action router finishes dispatch
- `Preconditions`: selected action variant is valid in current scope
- `Immediate Result`: control is handed to downstream flows such as settings, open-doc, branch-switch, merge-peer, or repo file operations
- `Application Entry`: `apps/web/src/components/search_box/logic/execute.rs`, `apps/web/src/components/command_palette/registry.rs`

## Response Flow

1. User confirms a result from the shared command surface.
2. Instruction interface resolves that result into a typed `SearchAction`.
3. Flow coordination dispatches the action variant to its downstream target contract.
4. Execution domains touched here are `commands`, `tree`, and `protocol`, because the router hands control into repo-scoped or runtime-scoped flows without inventing a second dispatch path.

## Notes

- This flow is the shared bridge after selection and before each concrete target flow takes over.
- It does not replace `command-palette`, `open-doc`, or `branch-switch`; it explains the common routing layer they reuse.
- Main objects: `search::action`, `command::selection`, `command::handoff`.

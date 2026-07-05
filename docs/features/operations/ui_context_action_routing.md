# ui_context_action_routing.md - Context Action routing chain

## Metadata

- `Flow ID`: `flow.ui.context-action-routing`
- `Domain`: `ui-shell`
- `Related Feature Chapters`: `docs/features/08_ui_design.md`, `docs/features/08_ui_design_01_web.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `UI-WEB-007`

## Operations

### `op.ui.context-action.open-menu`

- `Name`: `Open Context Action Menu`
- `Surface`: `file-tree-item`
- `Trigger`: open the file tree context menu
- `Preconditions`: a repo-scoped file tree item is visible
- `Immediate Result`: the Web shell asks the Context Action registry for actions valid for the current surface, target, repo scope, and readiness
- `Application Entry`: `apps/web/src/components/sidebar_menu/`, `apps/web/src/context_action/catalog.rs`

### `op.ui.context-action.project-actions`

- `Name`: `Project Context Actions`
- `Surface`: `context-menu`
- `Trigger`: menu render requests projected actions
- `Preconditions`: action descriptors and `ContextActionReadiness` are available
- `Immediate Result`: scoped `ContextActionIntent` values are projected for actions that pass resolver admission
- `Application Entry`: `apps/web/src/context_action/projection.rs`, `apps/web/src/context_action/resolver.rs`

### `op.ui.context-action.choose-action`

- `Name`: `Choose Context Action`
- `Surface`: `context-menu`
- `Trigger`: select one projected menu action
- `Preconditions`: the selected action came from the current projection
- `Immediate Result`: the action intent is sent to the menu handler; no authority mutation has happened yet
- `Application Entry`: `apps/web/src/components/sidebar_menu/item.rs`, `apps/web/src/components/sidebar/item/action.rs`

### `op.ui.context-action.resolve-and-dispatch`

- `Name`: `Resolve And Dispatch Context Action`
- `Surface`: `workspace-shell`
- `Trigger`: the context action handler receives an intent
- `Preconditions`: current readiness, repo scope, target, and host action capabilities still match the intent
- `Immediate Result`: the handler re-runs resolver admission and dispatches only a resolved action to the downstream UI/backend/native route
- `Application Entry`: `apps/web/src/components/sidebar/item/action.rs`, `apps/web/src/context_action/resolver.rs`

## Response Flow

1. User opens the file tree context menu on a repo-scoped target.
2. Instruction interface builds a `ContextActionProjectionRequest` from the target, current scope, readonly/write readiness, and host action capabilities.
3. Flow coordination projects only descriptors that resolve for that surface and readiness, carrying a scoped `ContextActionIntent`.
4. On selection, the handler constructs a fresh resolve request from current readiness and calls the resolver again before dispatch.
5. Downstream execution remains owned by the target route: shell-local display actions stay local, backend/native actions use their typed backend or host adapter, and authority writes still go through their backend/core writer gates.

## Notes

- This flow is an admission and routing layer for context actions; it does not decide write success, ack state, pending state, Source Control state, ledger facts, or Git mirror state.
- It keeps the frontend thin by centralizing action availability in the Context Action registry/resolver instead of embedding business rules in sidebar menu components.
- Main objects: `context_action::descriptor`, `context_action::intent`, `context_action::readiness`, `context_action::resolver`.

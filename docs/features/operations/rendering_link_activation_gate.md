# rendering_link_activation_gate.md - 链接激活闸门链

## Metadata

- `Flow ID`: `flow.rendering.link-activation-gate`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`
- `Related Acceptance Cases`: `RENDER-LINK-001`

## Operations

### `op.render.link.press-modifier`

- `Name`: `Press Ctrl/Cmd Modifier`
- `Surface`: `workspace-shell`
- `Trigger`: press `Ctrl` or `Cmd`
- `Preconditions`: editor shell is focused
- `Immediate Result`: global modifier state arms link activation styling without navigating
- `Application Entry`: `apps/web/src/hooks/use_ctrl_key.rs`, `apps/web/src/components/main_layout.rs`

### `op.render.link.click-armed-link`

- `Name`: `Click Armed Link`
- `Surface`: `editor`
- `Trigger`: left-click a link while `Ctrl/Cmd` is pressed
- `Preconditions`: modifier state is armed and clicked token resolves to a link URL
- `Immediate Result`: link click plugin opens the URL through the guarded path instead of plain cursor movement
- `Application Entry`: `apps/web/js/extensions/hyperlink_click.js`

### `op.render.link.release-modifier`

- `Name`: `Release Ctrl/Cmd Modifier`
- `Surface`: `workspace-shell`
- `Trigger`: release `Ctrl/Cmd` or blur the window
- `Preconditions`: modifier state is currently armed
- `Immediate Result`: link activation styling and navigation arm are cleared
- `Application Entry`: `apps/web/src/hooks/use_ctrl_key.rs`

## Response Flow

1. User presses `Ctrl/Cmd`.
2. Instruction interface arms link activation state and waits for a guarded click.
3. Flow coordination resolves the clicked link and opens it through the safe external path.
4. Execution domain is rendering projection with UI state gated by modifier state.

## Notes

- Ordinary clicks must remain editing actions, not navigation.
- Main objects: `render::projection`, `editor::selection`, `link::activation`.

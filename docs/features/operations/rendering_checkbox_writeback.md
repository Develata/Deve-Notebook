# rendering_checkbox_writeback.md - 任务列表源码回写链

## Metadata

- `Flow ID`: `flow.rendering.checkbox-writeback`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`
- `Related Acceptance Cases`: `RENDER-RICH-001`

## Operations

### `op.render.checkbox.click-toggle`

- `Name`: `Click Task Checkbox`
- `Surface`: `editor`
- `Trigger`: click a rendered task-list checkbox widget
- `Preconditions`: task marker is visible and editor is writable
- `Immediate Result`: checkbox widget dispatches a source edit rather than mutating an independent rich state
- `Application Entry`: `apps/web/js/extensions/checkbox_ext.js`

### `op.render.checkbox.observe-widget`

- `Name`: `Observe Checkbox Widget State`
- `Surface`: `editor`
- `Trigger`: render cycle completes after the click
- `Preconditions`: source text now contains the updated task marker
- `Immediate Result`: checkbox widget reflects the new checked state from source
- `Application Entry`: `apps/web/js/extensions/checkbox_ext.js`

### `op.render.checkbox.observe-source`

- `Name`: `Observe Task Source Writeback`
- `Surface`: `editor`
- `Trigger`: cursor returns to the task source or source is inspected after click
- `Preconditions`: click dispatch succeeded
- `Immediate Result`: Markdown source shows `- [x]` or `- [ ]` as the single authority state
- `Application Entry`: `apps/web/src/editor/delta_input.rs`, `apps/web/src/editor/`

## Response Flow

1. User clicks a rendered task checkbox.
2. Instruction interface converts the click into an editor change.
3. Flow coordination writes the task marker back into Markdown source and rebuilds the checkbox widget from source.
4. Execution domains are rendering projection and ledger-backed document content.

## Notes

- Checkbox interaction is allowed only because it round-trips through source text.
- Main objects: `doc::content`, `render::projection`, `task::checkbox`.

# rendering_projection_refresh.md - 投影刷新链

## Metadata

- `Flow ID`: `flow.rendering.projection-refresh`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`
- `Related Acceptance Cases`: `RENDER-BLOCK-001`, `RENDER-INLINE-001`, `RENDER-CURSOR-001`

## Operations

### `op.render.refresh.edit-source`

- `Name`: `Edit Source Span`
- `Surface`: `editor`
- `Trigger`: type inside a revealed inline, math, or diagram source range
- `Preconditions`: authoritative Markdown source is editable
- `Immediate Result`: editor content changes first; render state becomes pending recompute
- `Application Entry`: `apps/web/src/editor/delta_input.rs`, `apps/web/src/editor/delta_input_forward.rs`

### `op.render.refresh.leave-range`

- `Name`: `Leave Edited Render Range`
- `Surface`: `editor`
- `Trigger`: move cursor outside the recently edited renderable range
- `Preconditions`: edited source remains syntactically renderable
- `Immediate Result`: projection path becomes eligible to re-render from source
- `Application Entry`: `apps/web/js/editor_adapter.js`, `apps/web/src/editor/`

### `op.render.refresh.observe-updated-view`

- `Name`: `Observe Updated Projection`
- `Surface`: `editor`
- `Trigger`: render cycle completes after source edit
- `Preconditions`: render extension accepts the updated source
- `Immediate Result`: visible projection reflects the latest source without creating a second text authority
- `Application Entry`: `apps/web/js/extensions/inline_renderer.js`, `apps/web/js/extensions/math.js`, `apps/web/js/extensions/mermaid.js`

## Response Flow

1. User edits authoritative Markdown source.
2. Instruction interface forwards editor deltas and selection transitions.
3. Flow coordination recomputes projection widgets and hidden-syntax decorations from source.
4. Execution domains are editor runtime, rendering projection, and ledger-backed document content.

## Notes

- Projection refresh is a derived-state update, never a separate persisted document.
- Main objects: `doc::content`, `render::projection`, `editor::selection`.

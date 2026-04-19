# rendering_cursor_reveal.md - Cursor Reveal 渲染链

## Metadata

- `Flow ID`: `flow.rendering.cursor-reveal`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`
- `Related Acceptance Cases`: `RENDER-CURSOR-001`, `RENDER-RICH-002`

## Operations

### `op.render.cursor.enter-token`

- `Name`: `Enter Rendered Token`
- `Surface`: `editor`
- `Trigger`: move cursor into math, emphasis, Frontmatter, quote, or list marker
- `Preconditions`: source-first editor is open
- `Immediate Result`: rendered decoration yields to source text for the active token
- `Application Entry`: `apps/web/js/extensions/hybrid.js`, `apps/web/js/extensions/math.js`, `apps/web/src/editor/`

### `op.render.cursor.edit-source`

- `Name`: `Edit Revealed Source`
- `Surface`: `editor`
- `Trigger`: type while cursor is inside the revealed source span
- `Preconditions`: cursor reveal is active
- `Immediate Result`: source text changes rather than a detached rich-text projection
- `Application Entry`: `apps/web/src/editor/delta_input.rs`, `apps/web/src/editor/delta_input_forward.rs`

### `op.render.cursor.leave-token`

- `Name`: `Leave Rendered Token`
- `Surface`: `editor`
- `Trigger`: move cursor outside the active token range
- `Preconditions`: source text remains syntactically renderable
- `Immediate Result`: visual projection may reappear without changing authority text
- `Application Entry`: `apps/web/js/extensions/hybrid.js`, `apps/web/js/extensions/math.js`

## Response Flow

1. User moves the cursor into a rendered token.
2. Instruction interface receives selection/cursor updates from editor runtime.
3. Flow coordination compares cursor range with renderable spans and switches the active span to source mode.
4. Execution domains are rendering projection, editor runtime, and ledger-backed document content.

## Notes

- Cursor reveal is a projection rule; it must not create a parallel rich-text authority.
- Main objects: `doc::content`, `render::projection`, `editor::selection`.

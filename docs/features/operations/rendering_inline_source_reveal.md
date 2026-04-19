# rendering_inline_source_reveal.md - 行内源码揭示链

## Metadata

- `Flow ID`: `flow.rendering.inline-source-reveal`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`
- `Related Acceptance Cases`: `RENDER-CURSOR-001`, `RENDER-RICH-002`

## Operations

### `op.render.inline.enter-markup`

- `Name`: `Move Cursor Into Inline Markup`
- `Surface`: `editor`
- `Trigger`: move cursor into emphasis, strike, quote, link syntax, or inline code markup
- `Preconditions`: source-first editor is open and syntax hiding is active
- `Immediate Result`: hidden inline syntax for the active token becomes visible
- `Application Entry`: `apps/web/js/extensions/hybrid.js`, `apps/web/src/editor/`

### `op.render.inline.enter-frontmatter`

- `Name`: `Move Cursor Into Frontmatter`
- `Surface`: `editor`
- `Trigger`: move cursor into the Frontmatter block
- `Preconditions`: document contains a valid Frontmatter range
- `Immediate Result`: Frontmatter delimiters and block boundaries are revealed for direct editing
- `Application Entry`: `apps/web/js/extensions/hybrid.js`

### `op.render.inline.leave-markup`

- `Name`: `Move Cursor Outside Inline Markup`
- `Surface`: `editor`
- `Trigger`: move cursor outside the active inline or Frontmatter range
- `Preconditions`: source remains renderable after the edit
- `Immediate Result`: inline decoration may resume, but source remains the only authority text
- `Application Entry`: `apps/web/js/extensions/hybrid.js`

## Response Flow

1. User moves the cursor into or out of hidden syntax.
2. Instruction interface receives editor selection updates.
3. Flow coordination toggles syntax-hidden and Frontmatter decoration state for the active range.
4. Execution domains are rendering projection and ledger-backed document content.

## Notes

- This flow governs reveal and restore of inline markup, not rich-text authority transfer.
- Main objects: `doc::content`, `render::projection`, `editor::selection`.

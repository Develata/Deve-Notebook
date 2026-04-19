# rendering_math_source_projection.md - 数学源码投影链

## Metadata

- `Flow ID`: `flow.rendering.math-source-projection`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `RENDER-MATH-001`, `RENDER-BLOCK-001`

## Operations

### `op.render.math.enter-source-range`

- `Name`: `Move Cursor Into Math Source`
- `Surface`: `editor`
- `Trigger`: move cursor into an inline or block math range
- `Preconditions`: document contains renderable math delimiters
- `Immediate Result`: math widget yields to raw `$...$` or `$$...$$` source
- `Application Entry`: `apps/web/js/extensions/math.js`, `apps/web/src/editor/`

### `op.render.math.type-latex-source`

- `Name`: `Type LaTeX Source`
- `Surface`: `editor`
- `Trigger`: type inside the active math source range
- `Preconditions`: editor is writable and math source is authoritative
- `Immediate Result`: LaTeX source changes without replacing source with rendered HTML
- `Application Entry`: `apps/web/src/editor/delta_input.rs`, `apps/web/js/extensions/math.js`

### `op.render.math.leave-source-range`

- `Name`: `Move Cursor Outside Math Source`
- `Surface`: `editor`
- `Trigger`: move cursor outside the active math range
- `Preconditions`: source remains parsable as math
- `Immediate Result`: KaTeX projection reappears from source while block sizing remains predictable
- `Application Entry`: `apps/web/js/extensions/math.js`, `apps/web/src/components/outline_render/katex.rs`

## Response Flow

1. User enters and edits a math source range.
2. Instruction interface receives selection and delta updates from the editor runtime.
3. Flow coordination preserves Markdown source and rebuilds the math widget from that source.
4. Execution domains are rendering projection, tech-stack dependency adapters, and ledger-backed document content.

## Notes

- Math rendering is a projection over source, not a rich-text replacement.
- Main objects: `doc::content`, `render::projection`, `editor::selection`, `tech::dependency`.

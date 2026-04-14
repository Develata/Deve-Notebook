# rendering_math_mermaid.md - Math 与 Mermaid 渲染链

## Metadata

- `Flow ID`: `flow.rendering.math-mermaid`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `RENDER-MATH-001`, `RENDER-MERMAID-001`, `RENDER-BLOCK-001`

## Operations

### `op.render.math.type-source`

- `Name`: `Type Math Source`
- `Surface`: `editor`
- `Trigger`: type inline or block LaTeX syntax
- `Preconditions`: editor is writable
- `Immediate Result`: source text remains editable and can render as math projection
- `Application Entry`: `apps/web/js/extensions/inline_renderer.js`, `apps/web/src/components/outline_render/katex.rs`

### `op.render.mermaid.type-source`

- `Name`: `Type Mermaid Source`
- `Surface`: `editor`
- `Trigger`: type a fenced `mermaid` code block
- `Preconditions`: editor is writable and Mermaid runtime is available
- `Immediate Result`: diagram projection renders from the fenced source
- `Application Entry`: `apps/web/js/extensions/inline_renderer.js`, `apps/web/package.json`

### `op.render.projection.edit-source`

- `Name`: `Edit Render Projection Source`
- `Surface`: `editor`
- `Trigger`: enter math or Mermaid source and edit text
- `Preconditions`: source-first reveal path is available
- `Immediate Result`: projection refreshes from source without replacing the source as authority
- `Application Entry`: `apps/web/src/editor/delta_input.rs`, `apps/web/js/extensions/inline_renderer.js`

## Response Flow

1. User types math or Mermaid source.
2. Instruction interface forwards editor deltas and render extension state.
3. Flow coordination keeps source text authoritative while rendering a derived projection.
4. Execution domains are editor runtime, rendering projection, and tech-stack adapter dependencies.

## Notes

- Mermaid and KaTeX are visual projections over Markdown source.
- Main objects: `doc::content`, `render::projection`, `tech::dependency`.

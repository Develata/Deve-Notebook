# rendering_mermaid_source_projection.md - Mermaid 源码投影链

## Metadata

- `Flow ID`: `flow.rendering.mermaid-source-projection`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `RENDER-MERMAID-001`, `RENDER-BLOCK-001`

## Operations

### `op.render.mermaid.enter-source-block`

- `Name`: `Move Cursor Into Mermaid Source`
- `Surface`: `editor`
- `Trigger`: move cursor into a fenced `mermaid` code block
- `Preconditions`: document contains a Mermaid fence
- `Immediate Result`: rendered diagram yields to raw Mermaid source
- `Application Entry`: `apps/web/js/extensions/mermaid.js`, `apps/web/src/editor/`

### `op.render.mermaid.type-diagram-source`

- `Name`: `Type Mermaid Diagram Source`
- `Surface`: `editor`
- `Trigger`: type inside the active Mermaid fence
- `Preconditions`: editor is writable and Mermaid runtime is available
- `Immediate Result`: authoritative diagram text changes while render widget stays inactive for the touched block
- `Application Entry`: `apps/web/src/editor/delta_input.rs`, `apps/web/js/extensions/mermaid.js`

### `op.render.mermaid.leave-source-block`

- `Name`: `Move Cursor Outside Mermaid Source`
- `Surface`: `editor`
- `Trigger`: move cursor outside the active Mermaid fence
- `Preconditions`: source remains parsable by Mermaid
- `Immediate Result`: diagram widget re-renders with stable height derived from source line count
- `Application Entry`: `apps/web/js/extensions/mermaid.js`

## Response Flow

1. User enters and edits a Mermaid source block.
2. Instruction interface receives selection and delta updates from the editor runtime.
3. Flow coordination swaps between fenced source and Mermaid widget without changing source authority.
4. Execution domains are rendering projection, tech-stack dependency adapters, and ledger-backed document content.

## Notes

- Mermaid rendering stays local to the bundled runtime and should not require network fetches.
- Main objects: `doc::content`, `render::projection`, `editor::selection`, `tech::dependency`.

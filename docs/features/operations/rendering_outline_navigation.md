# rendering_outline_navigation.md - Outline 跳转链

## Metadata

- `Flow ID`: `flow.rendering.outline-navigation`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`
- `Related Acceptance Cases`: `RENDER-OUTLINE-001`

## Operations

### `op.render.outline.toggle-panel`

- `Name`: `Toggle Outline Panel`
- `Surface`: `editor-shell`
- `Trigger`: click the outline toggle affordance
- `Preconditions`: editor shell is open and not embedded
- `Immediate Result`: outline visibility state changes without affecting document authority
- `Application Entry`: `apps/web/src/editor/mod.rs`, `apps/web/src/hooks/use_outline.rs`

### `op.render.outline.select-heading`

- `Name`: `Select Outline Heading`
- `Surface`: `outline-panel`
- `Trigger`: click a heading item in the outline
- `Preconditions`: heading projection has been parsed from current content
- `Immediate Result`: selected outline item requests editor scroll to the heading line
- `Application Entry`: `apps/web/src/components/outline.rs`

### `op.render.outline.observe-target`

- `Name`: `Observe Editor At Heading Target`
- `Surface`: `editor`
- `Trigger`: editor scroll request completes
- `Preconditions`: target heading line exists in current content
- `Immediate Result`: editor viewport lands on the requested heading while outline text remains a derived projection
- `Application Entry`: `apps/web/src/editor/mod.rs`, `apps/web/js/editor_adapter.js`

## Response Flow

1. User opens the outline and selects a heading.
2. Instruction interface resolves current outline projection and issues a scroll request.
3. Flow coordination keeps outline parsing and editor scroll synchronized to current document content.
4. Execution domains are rendering projection and ledger-backed document content.

## Notes

- Outline items are derived from headings and must not invent unsupported syntax semantics.
- Main objects: `doc::content`, `outline::projection`, `editor::selection`.

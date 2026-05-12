# rendering_large_doc_search_gate.md - 大文档搜索闸门链

## Metadata

- `Flow ID`: `flow.rendering.large-doc-search-gate`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`, `docs/features/08_ui_design.md`
- `Related Acceptance Cases`: `RENDER-LARGE-001`, `UI-DESK-003`

## Operations

### `op.render.large-search.open`

- `Name`: `Open Search While Large Document Loads`
- `Surface`: `keyboard-shortcut-or-sidebar`
- `Trigger`: open Unified Search during large-document loading
- `Preconditions`: workspace shell is visible
- `Immediate Result`: search surface can open even if the current document is still loading
- `Application Entry`: `apps/web/src/components/search_box/mod.rs`, `apps/web/src/components/main_layout.rs`

### `op.render.large-search.submit-during-load`

- `Name`: `Submit Search During Partial Load`
- `Surface`: `search-input`
- `Trigger`: submit a search query while `load_state != ready`
- `Preconditions`: large-document replay is still in progress
- `Immediate Result`: search request is blocked by the load gate and no `ClientMessage::Search` is sent
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks/misc.rs`

### `op.render.large-search.submit-after-ready`

- `Name`: `Submit Search After Load Ready`
- `Surface`: `search-input`
- `Trigger`: resubmit search after load state becomes `ready`
- `Preconditions`: workspace is ready and scope is stable
- `Immediate Result`: gate releases and search request proceeds into the normal search pipeline
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks/misc.rs`, `apps/cli/src/server/handlers/search.rs`

## Response Flow

1. User opens search while a large document is still loading.
2. Instruction interface checks `load_state` and scope stability before sending search.
3. Flow coordination blocks search during partial replay, then releases it once the document is ready.
4. Execution domains are rendering load state, search service, and protocol request dispatch.

## Notes

- The gate protects responsiveness; it is not just a UI hint.
- Main objects: `load::progress`, `search::gate`, `search::baseline-scan`.

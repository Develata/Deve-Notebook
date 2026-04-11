# repo_file_operations.md - 文档结构写操作流

## Metadata

- `Flow ID`: `flow.repo.file-operations`
- `Domain`: `repository`
- `Related Feature Chapters`: `docs/features/06_repository.md`, `docs/features/08_ui_design.md`
- `Related Acceptance Cases`: `REPO-FEAT-01`, `UI-DESK-003`

## Operations

### `op.repo.file-ops.open-surface`

- `Name`: `Open File Operation Surface`
- `Surface`: `search-box-or-sidebar`
- `Trigger`: SearchBox file command, Explorer context menu, or create action
- `Preconditions`: local repo scope is active
- `Immediate Result`: app prepares create / rename / copy / move / delete intent
- `Application Entry`: `apps/web/src/components/search_box/`, `apps/web/src/components/sidebar/`

### `op.repo.file-ops.type-path`

- `Name`: `Type File Operation Path`
- `Surface`: `search-input-or-dialog`
- `Trigger`: user types target path or confirms prefilled command
- `Preconditions`: operation surface is open
- `Immediate Result`: path draft is normalized and candidate action is built
- `Application Entry`: `apps/web/src/components/search_box/file_ops/`

### `op.repo.file-ops.submit-create`

- `Name`: `Submit CreateDoc`
- `Surface`: `search-box-or-explorer`
- `Trigger`: choose `CreateDoc` candidate or create control
- `Preconditions`: repo write gate allows local write
- `Immediate Result`: sends `ClientMessage::CreateDoc`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_doc_write_create.rs`

### `op.repo.file-ops.submit-path-op`

- `Name`: `Submit Rename Copy Move Delete`
- `Surface`: `search-box-or-explorer`
- `Trigger`: choose file op candidate or context-menu action
- `Preconditions`: repo write gate allows local write and target path is scoped
- `Immediate Result`: sends `RenameDoc`, `CopyDoc`, `MoveDoc`, or `DeleteDoc`
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_doc_write_path.rs`

### `op.repo.file-ops.receive-result`

- `Name`: `Receive File Operation Result`
- `Surface`: `explorer`
- `Trigger`: server applies document structure mutation
- `Preconditions`: request scope nonce matches current repo scope
- `Immediate Result`: tree / doc list projection refreshes
- `Application Entry`: `apps/cli/src/server/ws/route/docs.rs`

## Response Flow

1. User opens SearchBox file command or Explorer context menu.
2. App builds a create / rename / copy / move / delete action.
3. Write gate checks local repo, readonly state, and current scope nonce.
4. Web sends the corresponding document structure `ClientMessage`.
5. CLI routes through `route_docs` and applies the docs handler.
6. Ledger and tree projection update, then the client refreshes doc list state.

## Notes

- Rename is represented as the `MoveDoc` path operation in SearchBox prefill.
- These operations mutate document structure, not document text content.
- Spectator / remote views must fail closed before sending structure writes.

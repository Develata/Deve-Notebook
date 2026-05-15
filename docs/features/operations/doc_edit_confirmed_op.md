# doc_edit_confirmed_op.md - 正文编辑确认链

## Metadata

- `Flow ID`: `flow.doc.edit-confirmed-op`
- `Domain`: `document`
- `Related Feature Chapters`: `docs/features/03_rendering.md`, `docs/features/04_storage.md`, `docs/features/16_web_thin_client_ledger.md`
- `Related Acceptance Cases`: `RENDER-FEAT-01`, `STORAGE-FEAT-01`, `STORAGE-FEAT-02`, `WEBWRITE-FEAT-01`, `WEBWRITE-FEAT-02`, `WEBWRITE-FEAT-03`

## Operations

### `op.doc.edit.type-content`

- `Name`: `Type Editor Content`
- `Surface`: `editor`
- `Trigger`: CodeMirror text input
- `Preconditions`: document is open, editable, and not in playback
- `Immediate Result`: editor emits delta JSON
- `Application Entry`: `apps/web/src/editor/delta_input.rs`

### `op.doc.edit.forward-delta`

- `Name`: `Forward Editor Delta`
- `Surface`: `editor-runtime`
- `Trigger`: delta passes write gates
- `Preconditions`: branch/repo switch idle, handshake ready, writer ready, stable `scope_nonce`
- `Immediate Result`: sends one or more `ClientMessage::Edit`
- `Application Entry`: `apps/web/src/editor/delta_input_forward.rs`

### `op.doc.edit.receive-ack`

- `Name`: `Receive Edit Ack`
- `Surface`: `status-runtime`
- `Trigger`: server returns `ServerMessage::Ack`
- `Preconditions`: ack scope and `client_op_id` match pending local edit
- `Immediate Result`: pending local edit is cleared
- `Application Entry`: `apps/web/src/hooks/use_core/effects/message_dispatch_write.rs`

### `op.doc.edit.receive-new-op`

- `Name`: `Receive Confirmed NewOp`
- `Surface`: `editor-runtime`
- `Trigger`: server broadcasts `ServerMessage::NewOp`
- `Preconditions`: repo/branch/scope match current editor context
- `Immediate Result`: confirmed op is applied or recognized as local echo
- `Application Entry`: `apps/web/src/editor/sync/live.rs`

### `op.doc.edit.receive-reject`

- `Name`: `Receive Edit Rejected`
- `Surface`: `status-runtime`
- `Trigger`: server returns `ServerMessage::EditRejected`
- `Preconditions`: rejection scope matches current workspace
- `Immediate Result`: pending local edit is cleared and protocol error is surfaced
- `Application Entry`: `apps/web/src/hooks/use_core/effects/message_dispatch_protocol.rs`

## Response Flow

1. User edits text in the source-first editor.
2. Web converts CodeMirror delta into one or more ledger `Op` values.
3. Web records a pending local edit and sends `ClientMessage::Edit`.
4. CLI validates browser scope, repo scope, readonly state, writer identity, and doc existence.
5. CLI appends a generated client op to the local ledger.
6. Sync manager writes the workspace projection after ledger commit.
7. Server broadcasts `NewOp` and unicasts `Ack`; failures send `EditRejected`.
8. Web clears pending state on ack/reject and applies confirmed remote ops.

## Notes

- The visible editor buffer is `confirmed + pending`, not authority.
- `Ack` only clears pending state; `NewOp` carries the confirmed ledger op.
- Re-sending the same `(client_id, client_op_id)` with the same op returns the original ack; sending a different op with the same key is rejected.
- Workspace projection failure after ledger append is recoverable and must not roll back authority.

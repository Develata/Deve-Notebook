# doc_pending_navigation_guard.md - Pending 编辑离开保护链

## Metadata

- `Flow ID`: `flow.doc.pending-navigation-guard`
- `Domain`: `document`
- `Related Feature Chapters`: `docs/features/16_web_thin_client_ledger.md`, `docs/features/08_ui_design_02_desktop.md`, `docs/features/06_repository.md`
- `Related Acceptance Cases`: `WEBNAV-FEAT-01`, `WEBNAV-FEAT-02`, `WEBNAV-FEAT-03`

## Operations

### `op.doc.nav.request-doc-change`

- `Name`: `Request Document Change`
- `Surface`: `quick-open-or-tree`
- `Trigger`: choose a different document
- `Preconditions`: current document may have pending local edits
- `Immediate Result`: navigation either runs immediately or becomes pending
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks/doc/select.rs`

### `op.doc.nav.request-scope-change`

- `Name`: `Request Scope Change`
- `Surface`: `repo-or-branch-switcher`
- `Trigger`: choose repo, branch, or home while current doc is open
- `Preconditions`: current document may have pending local edits
- `Immediate Result`: scope action either runs immediately or becomes pending
- `Application Entry`: `apps/web/src/hooks/use_core/callbacks_switch/repo.rs`, `apps/web/src/hooks/use_core/callbacks_switch/branch.rs`, `apps/web/src/components/main_layout/callbacks.rs`

### `op.doc.nav.choose-stay`

- `Name`: `Choose Stay`
- `Surface`: `pending-navigation-modal`
- `Trigger`: click cancel / stay
- `Preconditions`: pending navigation modal is visible
- `Immediate Result`: pending navigation is cleared; current doc and pending edits remain
- `Application Entry`: `apps/web/src/components/pending_navigation_modal.rs`

### `op.doc.nav.confirm-leave`

- `Name`: `Confirm Leave`
- `Surface`: `pending-navigation-modal`
- `Trigger`: click continue
- `Preconditions`: pending navigation modal is visible
- `Immediate Result`: modal is cleared and stored navigation action runs
- `Application Entry`: `apps/web/src/components/pending_navigation_modal.rs`

### `op.doc.nav.clear-on-confirmed`

- `Name`: `Clear Guard On Confirmed Edit`
- `Surface`: `runtime-status`
- `Trigger`: last pending edit receives `Ack` or `EditRejected`
- `Preconditions`: current doc pending set becomes empty
- `Immediate Result`: pending navigation guard is cleared
- `Application Entry`: `apps/web/src/hooks/use_core/effects/{message_dispatch_write.rs,message_dispatch_protocol.rs}`

## Response Flow

1. User requests a doc, repo, branch, or home navigation while a document is open.
2. Instruction interface builds the target action but sends it through `guard_navigation`.
3. Flow coordination checks only the current document pending set.
4. If no current-doc pending edit exists, the stored action runs immediately.
5. If current-doc pending edits exist, `PendingNavigation` stores target and action.
6. User chooses `Stay`; modal state clears without discarding edits or changing doc.
7. User chooses `Continue`; modal clears and the stored navigation action runs.
8. A later `Ack` or `EditRejected` clears the guard when the current doc pending set becomes empty.

## Notes

- `Continue` means leaving the view, not confirming write success.
- `Stay` must preserve pending local edits.
- The guard reads only current-doc pending edits, not every pending edit in the workspace.
- Main objects: `pending_local_edit`, `pending_navigation`, `doc::content`, `repo::scope`.

# rendering_large_doc_prefetch.md - 大文档渐进预加载链

## Metadata

- `Flow ID`: `flow.rendering.large-doc-prefetch`
- `Domain`: `rendering`
- `Related Feature Chapters`: `docs/features/03_rendering.md`, `docs/features/16_web_thin_client_ledger.md`
- `Related Acceptance Cases`: `RENDER-LARGE-001`, `RENDER-LARGE-002`

## Operations

### `op.render.large-doc.open`

- `Name`: `Open Large Document`
- `Surface`: `quick-open-or-explorer`
- `Trigger`: choose a large document from the current repo
- `Preconditions`: repo scope is stable and open-doc request can be issued
- `Immediate Result`: editor enters loading flow instead of waiting for full replay before showing content
- `Application Entry`: `apps/web/src/editor/hook_open.rs`, `apps/web/src/editor/sync/snapshot.rs`

### `op.render.large-doc.observe-first-screen`

- `Name`: `Observe First Screen During Partial Load`
- `Surface`: `editor`
- `Trigger`: snapshot content arrives before all delta ops are replayed
- `Preconditions`: load state is `partial` and snapshot content is available
- `Immediate Result`: first screen content is visible while remaining ops replay in batches
- `Application Entry`: `apps/web/src/editor/sync/snapshot.rs`, `apps/web/src/editor/sync/snapshot_apply.rs`, `apps/web/src/components/bottom_bar/stats.rs`

### `op.render.large-doc.observe-ready`

- `Name`: `Observe Large Document Ready State`
- `Surface`: `editor`
- `Trigger`: history replay and pending overlay reconciliation complete
- `Preconditions`: snapshot and delta replay path finished successfully
- `Immediate Result`: load state becomes `ready` and incremental progress indicators clear
- `Application Entry`: `apps/web/src/editor/sync/history.rs`, `apps/web/src/editor/sync/snapshot_finish.rs`

### `op.render.large-doc.delta-fallback`

- `Name`: `Fallback From Failed Delta Replay`
- `Surface`: `editor`
- `Trigger`: a snapshot delta batch cannot be applied to the current editor view
- `Preconditions`: snapshot request still matches current repo, branch, request, and scope
- `Immediate Result`: replay stops; local version/history are not advanced until the full-content fallback is applied
- `Application Entry`: `apps/web/src/editor/sync/snapshot.rs`, `apps/web/src/editor/sync/snapshot_apply.rs`, `apps/web/js/editor_remote_ops.js`

## Response Flow

1. User opens a large document.
2. Instruction interface starts open-doc loading and accepts the initial snapshot.
3. Flow coordination shows snapshot content first, then replays remaining ops in adaptive batches.
4. The batch chain is one owned task: it yields between batches and retires on completion, stale generation, or apply failure without leaving per-batch timers behind.
5. Failed delta replay stops the batch chain and lazily reconstructs full snapshot content from the batch task's retained operations. Normal replay does not precompute that fallback. If adapter application still fails, the editor enters a read-only structured error state; only the initial snapshot adapter failure may reopen once automatically, and explicit Retry creates a new generation/request.
6. Execution domains are rendering projection and ledger-backed document content.

## Notes

- This flow is about snapshot-first visibility plus progressive replay, not a second authority buffer.
- Failed delta replay must not advance local version or history.
- Full-snapshot fallback is lazy and borrows confirmed delta operations during reconstruction instead of precomputing a second full document or cloning a second complete operation/history list.
- Failed pending/history/live replay must not enter Ready or resend pending edits.
- Main objects: `doc::content`, `load::progress`, `render::projection`.

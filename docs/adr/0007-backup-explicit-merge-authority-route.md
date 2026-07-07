# 0007. Backup explicit merge authority route

- Status: Proposed
- Date: 2026-07-07

## Context

The backup restore pipeline can already download, verify, decrypt, validate
plaintext schema, admit a `RestoreCandidate`, and explicitly import a complete
ledger-only candidate into an existing empty local repo. `explicit-merge` is
still fail-closed by contract and implementation.

Enabling `explicit-merge` is not a local backup transport change. It crosses
the RestoreCandidate admission gate, current repo scope, writer gate,
Source Control, merge conflict surfacing, and ledger append authority. The
existing peer merge path merges a single document from a remote shadow branch;
it does not define how a verified backup candidate containing many content and
structure facts becomes a readonly merge source for the current local branch.

The key unresolved decisions are:

- How to fold backup candidate ledger entries into a readonly candidate repo
  snapshot without replaying backup global sequence.
- How to compare candidate structure facts against current local structure
  facts when `NodeId` / `DocId` paths diverge.
- Which conflicts enter the existing diff/conflict surface, and which remain
  fail-closed until a richer structure merge UI exists.
- How a one-shot CLI flow or future UI flow proves current `scope_nonce`,
  candidate fingerprint, target repo/branch, and writer gate freshness.

## Decision

No implementation route is accepted yet. Before `explicit-merge` can be
implemented, the project must choose one of these routes:

1. **Conservative first-tag merge runtime.**
   Add a dedicated restore merge authority runtime under Repo Runtime /
   Source Control / Merge Runtime. It consumes only a verified
   `RestoreCandidate`, candidate fingerprint, current local repo scope, and a
   current writer gate. It folds the candidate into a readonly snapshot,
   computes a merge plan, appends only merge-result facts through authority
   storage, and never writes staging, commit anchors, Git mirror queue, backup
   runtime state, or Projection Workspace authority directly. First-tag scope
   is intentionally narrow: content-only non-conflicting document merges may be
   applied; structure changes, deletes, renames, snapshot refs, blob refs, and
   ambiguous ancestry become conflicts or fail-closed diagnostics.

2. **Plan-only merge preview for first tag.**
   Keep non-dry-run `explicit-merge` fail-closed, but add a verified merge
   planner that reports candidate fingerprint, affected docs, unsupported
   structure/blob/snapshot cases, and conflict count without appending ledger
   facts. This improves recovery diagnostics but does not complete the Backup
   full-function requirement.

3. **Defer explicit-merge from first formal tag.**
   Treat `explicit-import` as the only write-back restore path for the first
   tag and keep `explicit-merge` as a hard release blocker until the full merge
   authority path is accepted.

The recommended route is **Route 1**, with the first implementation limited to
safe content-only merge cases and strict fail-closed behavior for structure and
asset surfaces.

## Rationale

Route 1 is the only route that satisfies "Backup as a complete first-version
feature" without letting backup transport become authority. The narrow first
slice avoids pretending that structure merge, delete/rename semantics, or
asset restoration are solved before the UI and conflict model are ready.

Route 2 is lower risk for authority, but it leaves disaster recovery incomplete
for users who need to merge a backup into a live repo. It is acceptable only as
an intermediate evidence step, not as the final first-tag Backup posture.

Route 3 is the simplest implementation route, but it contradicts the current
first-tag requirement that Backup be included as a complete feature. It also
keeps a known fail-closed product entry visible in the release plan.

## User Impact

With Route 1, users can recover safe content differences from a verified backup
into the current repo while conflicts remain explicit and reviewable. Users do
not see backup metadata, provider ETags, plaintext payloads, or locator strings
as authority; they see candidate fingerprint, target repo/branch, write-gate
state, merge result, and conflict diagnostics.

With Route 2, users can inspect what would be merged but cannot finish the
restore-to-current-repo workflow. This is useful evidence, but it is not a
complete recovery feature.

With Route 3, users must restore into an empty repo and then manually reconcile
with the current repo outside the intended authority path. That is operationally
awkward and increases the chance of post-tag compatibility debt.

## Consequences

- `backup_restore_runtime` may continue to verify and admit candidates, but it
  must not own merge planning or ledger append decisions.
- A Route 1 implementation needs a dedicated runtime boundary, targeted tests,
  and baseline entries for STORE-028 before removing the fail-closed behavior.
- Structure merge, delete/rename conflict UX, asset refs, multi-pack partial
  ancestry, and multi-step UI candidate handles should remain fail-closed until
  explicitly contracted.
- Frontend and CLI surfaces may only submit typed intents. They must not prove
  writer-gate freshness themselves, construct candidate fingerprints, pass raw
  plaintext/provider metadata, or provide ledger facts; backend/core authority
  paths must rebind and revalidate `scope_nonce`, target repo/branch,
  candidate fingerprint, and writer gate before any merge effect.
- Source Control dirty state after merge must remain derived from commit anchor
  to ledger head; Backup must not create staging, commit anchors, or Git mirror
  queue entries.
- This ADR does not change current behavior. Non-dry-run `explicit-merge`
  remains fail-closed until a route is accepted and implemented.

## References

- docs/plan/06_backup.md
- docs/plan/05_diff_logic.md
- docs/features/06_repository.md
- docs/acceptance-cases/07_storage_repo.md
- docs/registry/runtime-skeleton-registry.md

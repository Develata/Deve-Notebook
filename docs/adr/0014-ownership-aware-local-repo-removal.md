# 0014. Ownership-aware local repository removal

- Status: Accepted
- Date: 2026-07-20
- Amended: 2026-07-21 (`A1-S + B1-S + C2′-S` safety refinement)

## Context

The existing unpublished repo lifecycle removes catalog membership but keeps the canonical local
Redb database and `.notegit` runtime. Directly deleting them is unsafe because Redb handles are
distributed across `RepoManager`, sessions and a process-global cache; Windows normally refuses an
open-file delete, while Unix may unlink a pathname that a stale handle can continue writing.

The product requirement is narrower than destroying a workspace: remove every exact repo-scoped
object owned by Deve, while preserving Markdown, attachments, `.git`, unknown workspace files,
remote shadows, host identity and operator-provided recovery input. The last repo must also be
removable, leaving a useful zero-repo host.

## Decision

1. `authority_storage_runtime` gains one per-RepoId `RepoAuthoritySlot` as the only local Redb
   owner. Callers receive non-clone bounded leases; retirement closes admission only after provider
   quiesce and watcher E2, drains existing leases and retains an exclusive per-RepoId OS lock through
   cleanup and catalog retirement. The empty lock pathname is persistent host coordination identity,
   never a cleanup target. Same-RepoId readmission first reserves one map-level `Reopening` slot,
   then reacquires/revalidates outside the map mutex and installs one new generation by exact CAS.
2. Zero local repos is a valid `NoScope` host. Watcher expected=0 is healthy. First Create uses an
   explicit absolute `repo_creation_projection_base` when no current locator exists.
3. Remove is a two-phase backend flow. Prepare persists an exact ownership manifest and returns
   safe categories/blockers plus a random 256-bit, five-minute, one-time confirmation token. Only
   the token hash is stored. Execute consumes the token with exact membership, authority, scope,
   marker, locator and manifest bindings. Online tokens also bind authenticated principal,
   connection and server incarnation; offline tokens bind stable authority-root/lock identity rather
   than a short-lived CLI process. Execute atomically persists token consumption, execute request and
   job admission before starting work.
4. The unpublished F4/v4 wire target is replaced by F4/v5 lockstep. Direct Remove lifecycle intent
   is deleted; no v4 adapter or permanent replay fence is retained.
5. Committed cleanup is a recoverable saga. Exact unchanged remaining targets may resume at
   startup. Drift that is still provably owned by the same RepoId requires dry-run repair and a new
   explicit apply token. Textual RepoId markers cannot prove replacement ownership: replaced
   top-level `.notegit`, Redb, parent, unknown, escaping or unsafe-reparse identity is never applied.
6. `remote_import_runtime` remains the only owner of its artifacts and exposes a narrow typed
   removal plan/cleanup API. The plan is sealed before authority retirement; after the Removed cut,
   the owner performs artifact-only cleanup without mutating session rows in the retiring Redb. The
   lifecycle coordinator orchestrates owners but never deletes their paths or rows directly.
7. Before the Removed cut, any failure runs exact inverse compensation for authority, watcher,
   provider generation, sealed owner plans and Transitioning reservation. Compensation failure is
   a typed readonly/repair outcome with the write gate still closed. After cleanup, the durable
   terminal result is fsynced and the authority lock handle is released before best-effort
   session/network publication.
8. A fallback repo is an optional user choice made during Prepare. Execute can only echo the
   backend-issued opaque binding. Missing/stale fallback succeeds into `NoScope`; backend never
   chooses another repo. Final RepoList and scope are delivered as one typed finalization, never
   inferred from Source Control errors or message ordering.

## Consequences

- Bootstrap and secondary repos have identical authority lifetime and removal semantics.
- Workspace content is preserved, but local Ledger history and Deve runtime are irreversibly
  removed; the first release has no supported Ledger restore.
- Removal of the final repo no longer needs a fabricated fallback. A valid mounted fallback is an
  optional convenience; otherwise sessions enter `NoScope` and can create a new repo.
- The migration is broad inside existing runtime boundaries, but it does not change Ledger payload,
  Redb table schema, Projection format or sync facts.
- R1-R6 evidence must cover persistent lock identity, slot reincarnation, atomic admission/crash cuts,
  inverse compensation, single typed finalization, no-follow deletion, issuer-bound token replay,
  Remote Import states, zero-repo restart and real desktop/mobile browser flows before tag readiness.

## References

- docs/plan/03_storage/authority.md
- docs/plan/03_storage/index.md
- docs/plan/04_repository.md
- docs/plan/06_backup.md
- docs/plan/07_network.md
- docs/plan/13_i18n.md
- docs/plan/14_commands.md

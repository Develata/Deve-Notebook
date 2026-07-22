# Ownership-aware Repo Removal R4 Path-Safety Stop — 2026-07-22

- Status: `Approved by USER — D1-SQ + O1-FREEZE`
- Scope: R4 destructive settlement only
- Working baseline: `main@78b9265a9d8fa3ad96e2f6d5e541a27e2cecd244`
- Prior approved route: `A1-S + B1-S + C2′-S`
- Commit / push: none

## 1. Stop reason

The R4 implementation reached a functional owner-specific settlement path, but review found that
the destructive filesystem proof is not strong enough to publish or commit. Three cleanup owners
currently perform an identity check and later delete by pathname:

1. workspace `.notegit/` children and root;
2. the canonical Redb file;
3. the Remote Import artifact tree.

An uncooperative process can replace an intermediate ancestor with a symlink/junction between the
check and the unlink. The subsequent pathname operation can then delete an object outside the
sealed manifest. The persistent RepoId authority lock excludes cooperating Deve processes, but it
does not pin a filesystem pathname against an external actor.

This is a destructive-safety blocker, not a test-only gap. Retaining the current implementation can
violate the defining R4 invariant: remove only exact Deve-owned objects and preserve every object
outside the closed manifest.

## 2. Evidence

### 2.1 Pathname TOCTOU

- `.notegit`: `crates/core/src/utils/notegit/removal.rs` classifies an entry and later calls
  `remove_file` / `remove_dir` through the same pathname.
- Redb: `RepoAuthorityCleanupGuard::delete_database` classifies the expected DB identity and then
  calls `remove_file(db_path)`.
- Remote Import: the sealed inventory is reclassified per entry, then cleanup uses pathname-based
  `remove_file` / `remove_dir`.

The same race exists even when the final target identity is checked, because an intermediate parent
can change after that check.

### 2.2 Committed-cut debt can be lost

The catalog owner can persist the exact `Removed(request_id, manifest_digest)` cut before the
lifecycle receipt persists the observed tombstone. If transition into `CommittedCleanup` or the
receipt update then returns an error, the generic worker can terminalize a record whose
`RemovalExecutionState.tombstone` is still absent. That record is not classified as committed debt
and can eventually be pruned, while the catalog remains removed and cleanup no longer resumes.

### 2.3 Authority retirement can diverge from product success

The current code persists a successful terminal result and then calls
`RepoAuthorityCleanupGuard::complete()`. A failure is traced but does not suppress the `Removed`
publication. The client can therefore observe success while the slot and authority lock remain in
`CommittedCleanup`, preventing later same-RepoId admission and leaving no recoverable job.

### 2.4 Contract and projection drift

- The approved B1-S order seals owner plans before provider quiescence. The current implementation
  quiesces the provider and stops the watcher before sealing the Remote Import plan. This was done
  to stabilize the artifact tree, but it changes a fixed architecture order.
- The catalog record implementation is JSON v2 because the removed tombstone binds the manifest
  digest, while `03_storage/index`, `04_repository` and the first-tag matrix still say JSON v1.
- Current registry/overview wording overstates cold-start recovery, `Reopening` and single typed
  finalization evidence.

## 3. Decision D — destructive filesystem primitive

### D1-SQ — durable same-parent quarantine cut (recommended)

Each filesystem owner receives a narrow project-owned destructive primitive:

```text
Prepared(exact original, exact parent, reserved quarantine names) + fsync
-> atomic same-parent rename
-> verify moved object FileId/inode + parent identity + containment
-> Quarantined + fsync
-> delete only the verified quarantine object
-> CleanupComplete + fsync
```

Required rules:

- quarantine names are request/manifest-bound, reserved before destructive rename and never chosen
  by scanning or globbing;
- no cross-volume copy or fallback is allowed;
- recovery accepts `original exact + quarantine missing` before the quarantine checkpoint,
  `original missing + quarantine exact` after it, and `both missing` only when a durable
  quarantined checkpoint proves that deletion was the immediately preceding owner action;
  both-present, unproven both-missing or changed identity is `RepairRequired`;
- a later same-name `.notegit` or Redb object is never touched by the old job;
- `.notegit` moves its exact identity marker by same-filesystem, no-replace rename from a pinned
  source parent to a pinned workspace-root sibling quarantine, moves the tree to its own sibling
  quarantine, deletes the verified tree, then deletes the marker quarantine last and syncs the
  workspace parent;
- Remote Import moves the exact repo artifact root as one owner operation rather than unlinking
  individual inventory paths;
- recursive deletion is used only after quarantine identity verification and only on platforms
  where the selected runtime implementation is documented to resist symlink TOCTOU; unsupported
  platforms fail closed;
- every rename/delete/fsync cut is represented in the owner receipt and is restart-classifiable.

Benefits:

- one portable ownership model for `.notegit`, Redb and Remote Import;
- compact coordinator: it consumes typed owner dispositions rather than path details;
- crash recovery and same-name re-admission become mechanically distinguishable.

Costs and failure modes:

- adds durable owner intermediate states and reserved sidecar names;
- temporary quarantine objects can remain after a crash and require exact recovery/repair;
- Windows rename/open-handle behavior and junction replacement need real-process tests;
- marker-last cleanup requires a separate, durable marker-quarantine cut.

### D2-HR — platform handle-relative deletion

Implement Unix `openat`/`unlinkat`-style and Windows handle-relative native deletion behind one
project-owned adapter. This provides the strongest direct unlink semantics without pathname
quarantine, but creates substantially more unsafe/platform-specific code, auditing burden and test
surface. It is not recommended for the first public preview unless D1-SQ proves infeasible.

### D3-PATH — retain classify-then-delete

Rejected. Additional identity checks only narrow the race; they do not close the check/use window.

## 4. Decision O — Remote Import plan-sealing order

### O1-FREEZE — transition, provider quiesce, then seal (recommended)

Change the fixed pre-cut order to:

```text
ExecuteAdmitted + fsync
-> reserve Transitioning and close product write admission
-> exact static-owner revalidation
-> quiesce provider task and wait for in-flight capture to seal/abort
-> seal and persist the exact Remote Import cleanup plan
-> watcher E2 final reconcile
-> authority Quiescing/drain/exclusive retirement proof
-> Removed cut
```

This makes the Remote Import plan a snapshot of a provider-stable artifact tree. Compensation
first restarts the exact watcher generation, invalidates the sealed plan, then resumes the exact
provider generation before reopening admission. Plan invalidation failure leaves provider and
product admission quiesced.

Benefit: the plan cannot become stale merely because a capture that existed before Transitioning
finishes during quiescence. Cost: it explicitly amends the approved B1-S ordering.

### O2-RESERVE — seal before quiesce with a provider freeze reservation

Keep the written B1-S order, but add a provider state/token that first blocks artifact publication,
then permits sealing, and finally drains the already-frozen task during the named quiesce step.

Benefit: preserves the textual order. Cost: adds another provider state, generation token and
compensation branch whose only purpose is to distinguish "frozen" from "quiesced". This is more
complex than O1-FREEZE and offers no product benefit.

### O3-LITERAL — seal while provider can still mutate

Rejected. It permits avoidable post-seal drift and can turn a normal pre-cut capture completion
into committed repair debt.

## 5. Required state-machine hardening independent of D/O choice

1. Persist `CutAttempted` before entering the conditional catalog cut and persist `CutObserved`
   immediately after an exact removed tombstone is returned or rediscovered. Neither state may be
   terminalized as ordinary no-debt failure.
2. Classify the catalog truth before generic worker terminalization. Exact
   `Removed(request_id, manifest_digest)` always reconstructs committed cleanup debt.
3. Use two-phase terminal finalization:
   - persist the terminal result with `authority_retirement_pending=true` and publication disabled;
   - retire the authority slot/release its lock;
   - persist `authority_retired=true` and only then permit best-effort publication.
   A retirement failure remains recoverable committed debt and never returns product success.
4. Align the catalog epoch to deterministic JSON v2 in all authoritative docs and release
   registries. No v1 compatibility adapter is required because no public version exists.
5. Downgrade evidence claims until a cold-host restart rebuilds state from disk rather than reusing
   one `AppState` and authority runtime.

## 6. Impact surface

- `authority_storage_runtime`: exact DB quarantine/retirement and restart classification;
- `remote_import_runtime`: stable provider cut plus whole-root artifact quarantine;
- `.notegit` owner: marker-last two-cut cleanup;
- `RepoLifecycleJobRuntime`: cut-attempt/observed debt and two-phase finalization;
- catalog format docs/registry: JSON v2;
- tests: adversarial race hooks, cold restart, cross-process Windows/Linux evidence;
- no change to Ledger envelope/payload, Redb table schema, Projection format, sync facts, WS v5 or
  frontend authority.

## 7. Migration and rollback

- No released data migration is required. Development catalog/lifecycle receipts with unsupported
  epochs may fail closed and be rebuilt from a clean development fixture.
- The approved route salvages the typed owner plans, conditional catalog/alias/locator cleanup and
  existing non-destructive tests, then replaces all three pathname deletion paths before any
  success claim. Until those replacements and their recovery tests pass, the R4 working tree must
  not be used for destructive production-like testing.

## 8. Verification required for implementation

- deterministic race injection at classify/rename/unlink boundaries;
- original/quarantine both/missing/changed crash matrix for every owner;
- real Windows junction/reparse and Linux symlink ancestor replacement using a second process;
- DB open-handle and persistent authority-lock contention;
- cold-host restart after every catalog, owner receipt, quarantine and terminal-finalization cut;
- assertions for every deleted owner object and every preserved category, including attachments,
  unknown children, `.git`, ignore files, remote shadows, operator backups and other RepoIds;
- lost response across runtime restart and same-RepoId later admission;
- plan coverage, Markdown links, architecture registry, storage baseline, fmt, clippy and workspace
  tests on the same HEAD.

Chrome MCP remains not applicable to this backend R4 decision. Product-visible Remove/NoScope UI
and browser evidence belong to R5/R6.

## 9. Consequence of no change

R4 cannot be signed, committed as complete or used to unblock the first tag. Keeping the current
pathname cleanup risks deleting data outside the ownership manifest; keeping the current receipt
ordering can lose committed cleanup debt or publish success while the authority owner is still
locked.

## 10. Approved resolution

USER approved on 2026-07-22:

```text
D1-SQ + O1-FREEZE
```

together with the mandatory `CutAttempted/CutObserved` debt tracking, two-phase authority terminal
finalization and catalog JSON v2 documentation alignment in §5.

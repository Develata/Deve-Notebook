# Repo Reopening Authority Stop — 2026-07-22

Status: `Approved — Option A`

Exact base: `main@1aa6af9dc1072796e6f47e5d4b1ec9fea28efda3`

## Conclusion

R5 cannot safely sign off the currently specified in-process
`Retired -> Reopening -> Active(new_generation)` path without refining the
authority admission contract. The experimental implementation was fully
discarded and is not present in the worktree.

The recommended route is **A — two-stage, owner-prepared reincarnation**. It
keeps the persistent lock identity, catalog membership, prepared database
identity and authority generation in one typed admission choreography. It does
not change Ledger payloads, Redb tables, Projection format, sync facts or WS
F4/v5.

## Current Evidence

The existing implementation has these correct boundaries:

- `LocalAuthorityRuntime` retains a process-local `Retired { prior_generation }`
  slot after R4 settlement.
- catalog creation requires a `PreparedRepoAuthority` before it can publish a
  fresh durable `Normal` membership record.
- ordinary authority admission rejects `Retired` and does not silently reuse an
  old lease or generation.
- the authority lock pathname is intentionally persistent and is not an R4
  cleanup target.

The attempted narrow reopening implementation exposed five contract gaps:

1. **Producer cycle.** A fresh `Normal` membership cannot be committed without
   a prepared authority, while the existing creation path refuses to prepare
   an authority for a `Retired` slot. A token-only reopen is therefore either
   unreachable from production or requires bypassing the normal catalog cut.
2. **Lock identity loss.** The ordinary resource opener uses create-or-open for
   the persistent lock. Reopening must never recreate a missing lock pathname:
   another process may still hold the unlinked old inode, producing two
   independent owners.
3. **Insufficient identity proof.** A process-local membership token proves
   RepoId and catalog generation, but does not by itself prove that the opened
   DB, locator and marker match the durable `Normal` record's
   `prepared_identity_digest`.
4. **Validation currently mutates.** The existing `validate_existing` path can
   repair indexes and initialize Source Control tables. Reopening must not
   mutate the DB before final catalog identity revalidation.
5. **Post-CAS drift.** If a final membership check fails after publishing an
   `Active` slot, dropping the returned lease only decrements its count; it does
   not exact-rollback the slot and OS lock to `Retired`.

These are authority and runtime-boundary issues, not local implementation
defects. Patching only the observed call site would create an order-dependent
system in which some features can reopen a repo and others remain permanently
`Retired`.

## Option A — Two-Stage Owner-Prepared Reincarnation (Recommended)

Introduce one crate-private, non-clone authority capability and one composed
identity proof without transferring DB/lock ownership to the catalog. The
refined state choreography is:

```text
Retired(prior_generation, expected_lock_identity)
  -> Reopening(reservation_id, frozen_next_generation, expected_lock_identity)
  -> ReopeningPrepared(reservation_id, frozen_next_generation, prepared_authority)
  -> durable Normal catalog commit with fresh membership generation
  -> exact activation revalidation
  -> Active(prior_generation + 1)
```

### Prepare phase

- checked-add and freeze the next generation, then exact-CAS `Retired` to
  `Reopening`; the live Retired slot is the same-process terminal proof and
  does not depend on bounded receipt retention;
- require the durable catalog record to be absent;
- open the existing persistent lock with **no create and no symlink/reparse
  traversal**, then compare the opened handle/path to the Retired expected
  identity; a missing, replaced or unproven lock is `RepairRequired`;
- while holding that lock, require the canonical DB to be absent before a new
  incarnation is created;
- create and deterministically initialize the new DB through the authority
  owner, then exact-CAS to `ReopeningPrepared` and retain the non-clone
  `PreparedRepoAuthority`;
- let the composition layer combine DB physical/genesis identity, lock
  identity, locator store + exact row revision, and workspace root + marker
  identity into a project-owned `PreparedRepoIdentity`;
- do not expose a lease and do not publish `Active`.

### Catalog cut

- reuse the existing prepared creation and ordered catalog-cut APIs;
- commit a fresh deterministic `Normal` record and process membership
  generation bound to the prepared identity;
- retain the `ReopeningPrepared` reservation and persistent lock through this
  cut.

### Activation phase

- retain the lifecycle Transitioning permit, then acquire fixed-order locator
  read capability, catalog activation guard and authority slot capability so
  every project-owned identity owner is frozen through the final CAS;
- within the composed guard only bounded no-follow DB/lock/locator/marker
  identity revalidation is allowed, never repair/write/scan;
- exact-revalidate the fresh membership token, durable `Normal` record,
  recomputed identity digest and reservation;
- exact-CAS the same reservation to `Active(prior_generation + 1)` while the
  catalog guard is held;
- only after the CAS may existing-DB repair or mutating index upgrade run
  through normal admitted authority APIs. Deterministic new-DB schema/table
  initialization remains a required prepare-phase operation.

### Failure semantics

- a failed or unknown catalog call is classified under the catalog process
  lock: only exact Normal absence permits pre-cut owner-specific rollback;
  exact matching Normal continues activation/recovery, and unreadable/mixed
  truth is repair debt;
- pre-cut rollback may only invoke conditional DB, locator and marker owner
  cleanup; it never deletes the persistent lock, workspace content or another
  RepoId;
- after a committed catalog cut, activation failure becomes lock-held typed
  repair debt; it must not guess rollback, delete the new DB, restore old
  membership or publish a lease;
- panic/drop before activation releases the prepared DB and lock only after
  exact reservation rollback or marks repair if rollback cannot be proven;
- old membership tokens, removal tokens, cleanup capabilities and leases remain
  invalid by generation and reservation identity.

### Cost and impact

- adds a bounded state and capability to the existing
  `authority_storage_runtime` and `repo_catalog_runtime`;
- requires splitting pure existing-DB identity validation from repair writes;
- requires an existing-only persistent-lock opener;
- does not require a new crate, wire message, durable schema or compatibility
  adapter;
- provides the clean long-term producer for future explicit same-RepoId import
  or readmission while a live Retired proof exists, without coupling it to
  Remote Import or an accidental command order. Durable lineage after a fully
  removed host restart remains a separate future decision and fails closed.
- this work's only reachable producer is a typed server-composition
  `RepoLifecycleCoordinator::readmit_retired_repo` used by the production
  runtime and integration harness; it does not add a WS/CLI/UI trigger. Current
  Create remains fresh-UUID-only.

## Option B — Restart-Only Readmission

Keep `Retired` terminal for the life of the current process. Reject every
same-RepoId admission until the host restarts, at which point a new composition
root may bootstrap from a newly committed durable `Normal` record.

Benefits:

- smallest implementation and lowest immediate authority risk;
- no new in-process state choreography.

Costs:

- requires revising the current R5 contract;
- prevents clean same-process create/import of a preserved RepoId;
- pushes lifecycle complexity into operator restart and bootstrap recovery;
- is less suitable as the long-term architecture even if acceptable for a
  Public Preview.

## Rejected Option — Token-Only Reopening

Do not reopen solely from `CatalogMembershipToken + RepoInfo.uuid + schema`.
RepoId equality is necessary but not sufficient authority proof. This option
cannot prove persistent lock continuity, durable prepared identity, locator or
marker binding, and it permits DB mutation before final catalog admission.

## Failure Modes and Rollback

For Option A, implementation can be reverted before the catalog cut by exact
reservation cleanup. Once a fresh `Normal` record commits, rollback is not an
automatic product behavior: any activation failure remains repair debt and is
diagnosed from catalog, lock, DB, locator and marker truth.

If no change is made, the current system remains fail-closed and safe, but
same-RepoId in-process readmission remains unavailable. R5/R6 and first-tag
sign-off must continue to list this as an explicit gap.

## Required Verification for Option A

- real catalog remove, tombstone retirement, same-RepoId prepare, fresh Normal
  commit and production authority bind;
- old token, old bound authority, old cleanup capability and old request replay
  all fail;
- missing/replaced/unlinked authority lock fails without creating a new lock;
- DB, locator, marker and prepared identity mismatch each exact-roll back or
  enter repair according to the durable cut;
- two concurrent reopeners produce one reservation and one typed busy result;
- panic at every prepare/cut/activation boundary preserves a single owner;
- no DB repair/index mutation occurs before final activation;
- generation exhaustion and final catalog revalidation failure never leave an
  unintended `Active` slot;
- cold rebuild after Normal fsync but before Active recomputes DB, locator,
  marker and lock identity before admission; catalog-absent residual DB fails
  closed;
- Windows second-process lock/reparse and Linux inode/unlink tests.

## Decision

The USER approved **Option A** on 2026-07-22. The live contract is projected
into `docs/plan/03_storage/authority.md` and `docs/plan/04_repository.md`.
Implementation must preserve the stop conditions above and keep the first-tag
gate blocked until the required production-path evidence is sealed.

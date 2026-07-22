# Repo Reopening Authority Stop — 2026-07-22

Status: `Requires user decision`

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

Introduce one crate-private, non-clone capability owned jointly by the existing
catalog and authority runtimes. The state choreography is:

```text
Retired(prior_generation)
  -> ReopeningPrepared(reservation_id, prior_generation, prepared_authority)
  -> durable Normal catalog commit with fresh membership generation
  -> exact activation revalidation
  -> Active(prior_generation + 1)
```

### Prepare phase

- exact-CAS the runtime slot from `Retired` to `ReopeningPrepared`;
- require the old removal job to be terminal and the durable catalog record to
  be absent;
- open the existing persistent lock with **no create and no symlink/reparse
  traversal**; a missing or replaced lock is `RepairRequired`;
- while holding that lock, require the canonical DB to be absent before a new
  incarnation is created;
- create the new DB through the authority owner and produce a
  `PreparedRepoAuthority` plus project-owned `PreparedRepoIdentity`;
- do not expose a lease and do not publish `Active`.

### Catalog cut

- reuse the existing prepared creation and ordered catalog-cut APIs;
- commit a fresh deterministic `Normal` record and process membership
  generation bound to the prepared identity;
- retain the `ReopeningPrepared` reservation and persistent lock through this
  cut.

### Activation phase

- reacquire the short catalog cut guard;
- exact-revalidate the fresh membership token and durable `Normal` record;
- recompute the opened DB + locator + marker identity through a **pure**
  validation path and compare its digest with the record;
- exact-CAS the same reservation to `Active(prior_generation + 1)` while the
  catalog guard is held;
- only after the CAS may repair/index initialization run through normal
  admitted authority APIs.

### Failure semantics

- before the durable catalog cut: exact rollback removes only the newly
  prepared incarnation and restores `Retired`;
- after a committed catalog cut: activation failure becomes typed repair debt;
  it must not guess rollback, delete the new DB, or restore old membership;
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
  or readmission without coupling it to Remote Import or an accidental command
  order.

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
- Windows second-process lock/reparse and Linux inode/unlink tests.

## Decision Requested

Approve one of:

- **Option A**: refine the plan and implement two-stage owner-prepared
  reincarnation; or
- **Option B**: revise the plan to make same-RepoId readmission restart-only for
  the first release.

Recommendation: **Option A** for the cleanest long-term authority model.

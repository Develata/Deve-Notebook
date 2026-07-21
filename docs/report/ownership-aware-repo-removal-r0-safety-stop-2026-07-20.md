# Ownership-aware RemoveLocalRepo R0 safety stop — 2026-07-20

> This report records a review stop and its resolution. It is not a live contract. The USER approved
> the recommendation on 2026-07-21; authority now resides in `docs/plan/`.

## Metadata

- `Date`: `2026-07-20`
- `Status`: `Approved by USER — A1-S + B1-S + C2′-S`
- `Approved`: `2026-07-21`
- `Baseline`: `main@651dfc9e2` plus the uncommitted R0 contract draft
- `Trigger`: three-lane read-only R0 review
- `Unchanged direction`: `A1 + B1 + C2′`, zero-repo `NoScope`, F4/v5, preserve the workspace
- `Unchanged formats`: Ledger payload/envelope, Redb v4 tables, Projection format and sync facts

## 1. Why execution stopped

The R0 draft cannot be committed as an executable destructive contract yet. The approved intent
is sound, but several low-level rules cannot all be true at the same time:

1. the draft both quiesces authority before watcher E2 and requires E2 to finish while authority
   remains usable;
2. it deletes the pathname that supplies cross-process mutual exclusion;
3. it allows later admission of the same RepoId but gives a retired authority slot no safe new
   incarnation;
4. it drains ordinary authority leases before Remote Import owner cleanup, while the cleanup is
   still described as needing mutable session authority;
5. offline Prepare and Execute are separate CLI processes, but the token is process-bound;
6. token consumption and durable job admission do not have one atomic crash cut;
7. marker text alone is not sufficient proof that a replaced `.notegit` or Redb object still
   belongs to the removed membership;
8. `repo-scoped runtime artifacts` is an unsafe catch-all in a destructive manifest.

The current lock deletion rule is independently unsafe on both supported host families. On Unix,
unlinking a locked pathname permits a second inode to be created and locked while the old handle
still exists. On Windows, deletion normally fails while the lock handle is open; releasing it
before deletion creates a race with the next opener.

## 2. Recommended resolution: A1-S + B1-S + C2′-S

This is a safety refinement of the already approved route, not a new product feature.

### 2.1 A1-S — persistent coordination identity and slot reincarnation

- Keep `.host/repo-authority-locks/<repo_id>.lock` as an empty persistent host coordination
  object. It is never a removal target and contains no repo data.
- Hold its exclusive OS lock from before DB open through catalog tombstone retirement, all owner
  cleanup and durable terminal-result fsync. Release it before best-effort session/network
  publication so a disconnected observer cannot block later admission.
- A retired in-process slot first becomes one map-level
  `Reopening(reservation_id, prior_generation)` reservation. After the mutex is released, that
  opener reacquires the same lock pathname, revalidates current catalog/DB identity, and exact-CAS
  installs the new Active generation. Concurrent openers fail typed busy; failed reopen rolls the
  reservation back rather than leaving two owners.
- Process-only watcher/provider/session slots are stopped or retired through typed owner APIs.
  They are not filesystem manifest targets.

### 2.2 B1-S — seal owner plans before retirement; clean artifacts after the cut

Use one ordering everywhere:

```text
ExecuteAdmitted(token consumed + request/job bound) + fsync
-> close product mutation admission / reserve Transitioning
-> revalidate and seal owner-issued cleanup plans
-> quiesce provider tasks
-> watcher E2 final reconcile
-> authority Quiescing and drain ordinary leases (30 s, pre-cut timeout restores Active)
-> create owner-internal exclusive retirement capability
-> Removed tombstone + fsync
-> owner-specific artifact cleanup receipts + fsync
-> close/evict/delete canonical Redb
-> conditional locator and alias cleanup
-> CleanupComplete + fsync
-> exact tombstone retirement
-> durable terminal result + fsync
-> release the persistent authority lock handle
-> best-effort session/network publication
```

Remote Import must exact-check its state and seal an immutable removal plan before authority
retirement. After the Removed cut, its owner API performs artifact-only cleanup from that plan;
it does not mutate a session table that will be deleted with the canonical Redb. `Applied/Pending`,
`Applied/Degraded`, corrupt and unknown states retain their existing blockers.

The destructive manifest is closed and typed. Its only filesystem targets are owner-issued Remote
Import capture entries, the exact `.notegit` root and the exact canonical Redb file. Locator and
alias rows use conditional owner APIs; catalog tombstone and lifecycle records use their own store
APIs. Projection Fault and Remote Import session rows are part of the Redb target, not additional
paths. Future owners must register a typed removal plan before their artifacts can enter removal.

Terminal execute results remain under the existing bounded receipt policy so a lost response can
be replayed. They are not deleted as part of the same cleanup job.

Any failure before the `Removed` cut performs inverse compensation for the exact generation:
restore authority Active, restart the old watcher, restore the exact provider generation,
invalidate sealed owner plans and release the Transitioning reservation. If any compensation step
fails, the repo stays typed readonly/repair with its write gate closed; it is never reported as an
ordinary non-committed Active repo.

### 2.3 C2′-S — atomic admission and issuer-specific token binding

- Execute performs one durable CAS from `ManifestPrepared` to
  `ExecuteAdmitted { execute_request_id, job_id, consumed_token_hash }`, then fsyncs before a worker
  starts. Startup recovery resumes any admitted job; an exact retry returns that job/result.
- Prepare and Execute have distinct request IDs; Execute names the exact preparation record.
- An optional fallback is selected by the user during Prepare. The backend returns an opaque exact
  binding that Execute can only echo; missing or stale fallback produces a successful `NoScope`
  settlement rather than backend auto-selection.
- Online Web tokens additionally bind authenticated principal/session and connection epoch. The
  browser keeps the token in memory only; it never enters URL, browser storage or telemetry.
- A CLI routed through the running server binds the authenticated `LocalCliProxy` principal and
  the server runtime incarnation.
- Offline two-invocation CLI tokens bind the canonical authority root identity, the persistent
  authority-lock file identity, membership revision, authority generation and the preparation
  record. They do not bind the short-lived CLI process. This preserves the documented
  `preview -> --apply --token` workflow without weakening membership or five-minute expiry checks.

### 2.4 Repair proof boundary

- Replacement of the top-level `.notegit` identity is permanently blocked for automated repair,
  even if the replacement contains the same textual RepoId marker.
- A missing original target is treated as already cleaned. A remaining target is repairable only
  when its original owner-issued durable identity is unchanged and it remains inside the original
  containment boundary.
- Canonical Redb repair must match the original file identity and repository genesis/membership
  identity, not only a RepoId field.
- Child links/reparse entries under an unchanged `.notegit` root may be removed as entries without
  following targets. The external targets must remain untouched.
- An operator recovery input is guaranteed preserved only outside all reserved removal roots. Any
  active recovery input overlapping `.notegit`, canonical Redb or an owner capture target blocks
  Prepare.

## 3. Contract and evidence follow-up after approval

The corrected R0 must also:

- add stable plan anchors for the local removal contract and Remote Import removal-owner plan;
- model Prepare and Execute explicitly in the operation/Lisp architecture projection while
  retaining the current direct-Remove code node as honest drift until R3;
- bind all new STORE-014A safety assertions in `storage_repo.tsv`;
- use the single ADR status `Superseded by 0014` and retain the intermediate wire epochs only as
  decision history;
- add an explicit localized irreversible-removal confirmation key;
- distinguish top-level reparse blocker from safe no-follow deletion of child link entries.
- publish final RepoList and scope in one typed finalization; removal success is never inferred
  from a Source Control error or two-message ordering.

## 4. Cost, failure modes and rollback

The recommendation adds no crate, Redb table, Ledger fact, Projection format or sync message. It
does add one lifecycle-record state and requires owner-issued cleanup plans. Persistent empty lock
files accumulate at one tiny file per RepoId; this is intentional coordination metadata and avoids
an unsafe deletion race.

If not adopted, implementation can either deadlock/fail during watcher final reconcile, admit two
database owners for one RepoId, consume a token without a recoverable job, or delete a replaced
filesystem object on weak marker evidence. Those are unacceptable for a destructive first-public
operation.

Rollback before the Removed tombstone is the exact inverse-compensation sequence above, not merely
`Transitioning/Quiescing -> Active`. After the cut, membership is never restored; recovery resumes
the exact admitted job or exposes typed repair debt.

## 5. Verification after implementation

- Windows and Linux second-process lock/re-admission tests against the persistent pathname.
- concurrent same-RepoId reopen tests proving one `Reopening` reservation and one new generation.
- crash injection immediately before and after `ExecuteAdmitted` fsync and each later durable cut.
- pre-cut failure injection proving exact inverse compensation and readonly/repair fallback.
- watcher E2 test proving authority remains available until final reconcile completes.
- Remote Import owner-plan tests for every state and tampered capture identity.
- online Web, loopback CLI and two-process offline CLI token binding/replay tests.
- top-level replacement, child reparse, DB replacement and overlapping recovery-input tests.
- exact lost-response replay after terminal cleanup and same-RepoId re-admission.
- single typed RepoList/scope finalization with valid, stale and absent optional fallback bindings.

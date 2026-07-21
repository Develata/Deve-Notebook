# Ownership-aware RemoveLocalRepo architecture stop — 2026-07-20

> 本报告是 `docs/report/` 下的时点证据，不是 live contract。USER 已批准
> `A1 + B1 + C2′`，并于 2026-07-21 批准 `A1-S + B1-S + C2′-S` 安全细化；权威实现合同已投影到 `docs/plan/03_storage`、`04_repository`、
> `06_backup` 与 `07_network`。后续若报告文字与 live plan 冲突，以 plan 为准。

## Metadata

- `Date`: `2026-07-20`
- `Status`: `Approved — A1-S + B1-S + C2′-S`
- `Baseline`: `main@73ab0f4fe`
- `Approved product semantics`: delete local canonical Redb, exact workspace `.notegit`,
  locator row, alias row and transient catalog tombstone; preserve workspace root,
  Markdown/attachments, `.git`, remote shadows, explicit exports/backups and unrelated RepoIds
- `Explicitly unchanged`: Ledger envelope/payload, `DEVELDG3`, Redb v4 table schema,
  Projection format, sync facts and Remote Import ownership
- `Approved protocol cut`: unpublished WS F4/v4 -> first-public F4/v5, no v4 adapter

## 1. Current state and evidence

### 1.1 The bootstrap database cannot be retired safely

`RepoManager` currently owns an irreplaceable `local_db: Arc<Database>` and a secondary
database map. `AppState`, `SyncManager`, per-session database handles and the process-global
database cache retain additional `Arc<Database>` values. The serve path also chooses one
cataloged RepoId as a transitional bootstrap anchor.

The current soft remove only changes catalog membership and cleans the locator. It does not
have a way to revoke new database use, drain existing use, evict every cached handle, or prove
that another process cannot still write the file.

Deleting under the current composition has two platform-specific failure modes:

- Windows normally rejects deletion while a handle is open, leaving committed removal with
  residual authority state;
- Unix may unlink the pathname while an old handle continues to write the detached inode. A
  later database at the same logical identity can then create hidden split authority.

Treating either outcome as ordinary cleanup debt is unsafe. The DB must not be unlinked until
exclusive retirement is proven.

### 1.2 The destructive saga needs a durable cut order

The ownership manifest must survive before `Normal -> Removed`; otherwise a crash can leave a
tombstone without a safe cleanup candidate set. The required target sequence is:

```text
ManifestPrepared + fsync
  -> Removed(request_id, manifest_digest) + fsync
  -> exact per-item cleanup receipts + fsync
  -> CleanupComplete + fsync
  -> exact tombstone retirement
  -> durable terminal result + fsync
  -> release persistent authority lock handle
  -> best-effort terminal publication
```

Path targets must bind exact parent identity, containment and FileId/inode-style fingerprint.
Top-level DB and `.notegit` targets must not be symlink/junction/reparse objects. A no-follow
walker may delete a child link entry, but never its target. Locator, alias and catalog are
shared stores, so only their owners may execute revision-bound row cleanup.

### 1.3 Repair and retry are not yet closed

The current lifecycle contract says a restarted process does not resume the interrupted job.
That is correct for an uncommitted worker, but insufficient after a committed removal cut.
The repository needs a separate recovery owner that classifies the durable cut, never expands
the immutable manifest, and does not silently re-authorize a changed filesystem identity.

The existing public CLI has a general `repair` command, but no repo-removal request selector or
explicit cleanup-apply contract. The current wire has lifecycle lookup but no removal repair
intent. Therefore the operator trigger is an architecture/product-surface decision rather than
a missing helper function.

### 1.4 Bounded receipts permit old-request replay

Ordinary lifecycle terminal receipts are bounded. If a successful remove receipt is pruned and
the same logical RepoId is later re-admitted, an old `request_id` could otherwise be accepted
against the new membership. Safe removal needs a permanent minimal replay fence, a membership
incarnation/confirmation token, or an equivalent non-reuse proof.

## 2. Decision A — database authority ownership and retirement

| Route | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| A1 | Inside the existing `authority_storage_runtime`, introduce one per-RepoId `AuthorityRepoRuntime` as the only owner of the Redb handle. Callers receive bounded revocable leases. Retirement performs `Active -> Retiring -> Retired`, closes admission, drains leases, evicts the global cache and proves a cross-process exclusive removal lease before returning a typed deletion capability. | Uniform bootstrap/secondary behavior; no raw `Arc<Database>` escapes; deletion is mechanically safe on Windows and Unix; strengthens cohesion without adding a top-level crate/runtime. | Requires replacing distributed handle ownership across core/server/session paths and exhaustive cancellation/shutdown tests. A leaked lease blocks cleanup rather than being hidden. |
| A2 | Keep current handles; mark removal committed and defer DB deletion until a full server restart or offline maintenance. | Smaller initial code diff. | Product remove leaves Deve-owned state; crash/restart ordering remains ambiguous; another process still defeats safety; violates the requested semantics. |
| A3 | Forbid removal of the internal bootstrap repo and only retire secondary cache entries. | Avoids the hardest handle. | Leaks an implementation detail into product behavior, makes identical RepoIds behave differently and preserves the transitional anchor indefinitely. |

**Approved safety refinement: A1-S.** It remains inside the already named authority storage boundary.
The per-RepoId lock pathname is persistent host coordination identity and is never unlinked; only
its OS handle is released. A later same-RepoId admission replaces the Retired slot with a new
generation after reacquiring the same lock and revalidating membership. This does not create a new
public crate or move Ledger authority into the lifecycle coordinator.

The cross-process proof must be project-owned and acquired before unlink. Redb/open-file failure
alone is not the protocol. Any inability to obtain exclusive retirement produces typed cleanup
debt and leaves the DB untouched.

## 3. Decision B — committed cleanup recovery

| Route | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| B1 | Hybrid recovery: startup only classifies durable receipts. It may automatically continue a committed settlement when manifest digest, every remaining target fingerprint and containment are unchanged. Any drift becomes `RepairRequired`. Extend the existing repair surface with request-scoped dry-run output; actual cleanup requires explicit `--apply`. Server-held authority uses the same internal coordinator/proxy rather than opening Redb directly. | Exact unchanged crashes self-heal; changed identities never receive implicit destructive authority; operator sees the exact remaining set. | Adds a narrow repair producer and receipt state machine; must prove exclusivity with normal lifecycle jobs. |
| B2 | Never auto-continue. Every post-crash remaining target requires explicit repair apply. | Simplest destructive policy and easiest audit. | Normal crash recovery leaves avoidable long-lived tombstones and manual work even when all identities are unchanged. |
| B3 | Automatically retry every path in the saved manifest until absent. | Maximum apparent availability. | A replaced ancestor/target can receive stale deletion authority; violates fail-closed ownership. |

**Approved safety refinement: B1-S.** “Restart does not resume the old job” remains true; a separate recovery owner
settles exact unchanged committed truth. Dry-run is read-only. Apply cannot expand the original
manifest. A changed target may receive a short repair token only when its original owner-issued
durable identity and containment remain exact. A matching textual RepoId marker does not authorize
a replaced `.notegit`, Redb or parent identity; unknown/mismatched identity remains permanently blocked.

## 4. Decision C — destructive request replay protection

| Route | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| C1 | Retain a minimal host-global removal replay fence keyed by `request_id`, exact RepoId, removed membership revision and terminal outcome digest. | No WS change. | Permanently retains control-plane metadata and duplicates the authorization lifetime already expressible by a prepared removal. |
| C2′ | Add a prepare-issued 256-bit, five-minute, one-time confirmation token bound to membership revision, authority generation, locator/marker/manifest, scope and issuer identity. Online issuers bind principal/connection/server incarnation; offline CLI binds stable authority-root/lock identity rather than its short-lived process. Repeated Prepare invalidates the old token; raw token is never persisted. | Self-contained destructive authorization, backend preview, no permanent request history, and old requests cannot target a later membership. | Requires one pre-publication cut from F4/v4 to F4/v5 and lockstep client migration. |
| C3 | Use the current bounded lifecycle receipt only. | No new mechanism. | Unsafe once the receipt is pruned and the RepoId is re-admitted. |

**Approved safety refinement: C2′-S.** No public version exists, so the one-time F4/v5 cut is cheaper
and cleaner than retaining an unbounded replay fence. Execute atomically persists token consumption,
execute request and job admission before starting work. Exact idempotency uses the active/terminal
lifecycle receipt while it exists; after receipt retirement an old request has no valid current token.

## 5. Combined recommendation

USER approved:

```text
A1-S + B1-S + C2′-S
```

The resulting ownership direction is:

```text
RepoLifecycleCoordinator
  -> authority_storage_runtime retirement capability
  -> owner-specific exact cleanup commands
  -> durable cleanup receipt / tombstone retirement

UI / WS handler
  -> typed Prepare/Execute intent and typed outcome only
```

The lifecycle coordinator orchestrates; it does not own Redb, locator, alias, Remote Import
artifacts or filesystem inventory. The frontend renders the backend confirmation/outcome and
does not calculate the deletion set.

## 6. Impact surface

- `crates/core/src/ledger/manager/types.rs`
- `crates/core/src/ledger/database.rs`
- `crates/core/src/ledger/database_cache.rs`
- `crates/core/src/ledger/manager/authority_storage_runtime.rs`
- `apps/cli/src/server/state.rs`, session database ownership and repo lifecycle runtimes
- repo catalog, locator and host alias owner APIs
- owned `.notegit` remover and Windows/Linux no-follow fixtures
- lifecycle receipts, startup classification, repair command/proxy and thin Web confirmation

This is a broad call-relationship change inside an existing runtime boundary. It does not alter
Ledger payload, Redb v4 tables, Projection format or sync facts.

## 7. Migration and rollback

- No released compatibility adapter is required. Replace raw long-lived DB handle access in
  cohesive slices; do not keep old and new authority paths in parallel.
- First introduce the owner/lease API while retaining current behavior, migrate all callers,
  prove no raw handle escape, then enable retirement and physical cleanup.
- The destructive path remains blocked until every caller has migrated. A partial migration must
  not expose a UI that claims physical cleanup.
- Each slice is independently revertible before the deletion capability is enabled. Once an
  actual remove has committed, rollback is operationally fail-closed through its durable
  manifest/tombstone; code rollback must not guess restoration.

## 8. Verification after approval

- bootstrap and secondary RepoIds have identical admission/retirement behavior;
- deterministic barriers prove no new lease after `Retiring`, in-flight drain and shutdown;
- a second process holding/opening the DB blocks deletion on Windows and Unix;
- every manifest/tombstone/per-item/CleanupComplete crash cut is classified from durable truth;
- parent identity, containment, target fingerprint and reparse drift fail closed;
- no-follow deletion never enters link targets and preserves every non-`.notegit` workspace child;
- locator/alias/catalog conditional cleanup preserves unrelated RepoId rows;
- active Remote Import/cleanup debt remains an owner-reported blocker;
- repeated Prepare invalidates prior token; expired/stale/wrong-runtime token fails closed;
- old request replay cannot affect a re-admitted same RepoId after receipt retirement;
- real backend Chrome MCP proves irreversible confirmation, partial repair projection and thin UI;
- fmt/clippy/workspace tests, WASM, storage baseline, 434-row acceptance matrix, plan coverage,
  Markdown links and architecture diff all pass on the same HEAD.

## 9. Consequences of no change

- the current soft remove continues leaving local Redb and `.notegit` state;
- directly adding physical deletion risks Windows failure or Unix detached-authority split;
- a crash can leave an unrepairable tombstone if manifest durability is not ordered first;
- a stale path or replaced reparse target can receive unintended deletion authority;
- bounded receipt pruning can let an old remove request affect a later membership incarnation;
- `STORE-014A`, `flow.repo.lifecycle`, runtime registry convergence and first-tag readiness remain
  explicit gaps.

## 10. USER decision

Approved on 2026-07-20: `A1 + B1 + C2′`, together with zero-repo/`NoScope`, owner-coordinated
Remote Import cleanup and the first-public F4/v5 cut. No permanent replay fence is authorized.
Approved safety refinement on 2026-07-21: `A1-S + B1-S + C2′-S`; the persistent lock pathname,
provider/E2-before-Quiescing order, atomic ExecuteAdmitted cut, issuer-bound token and strict
owner-issued repair proof are now part of the live plan. The refinement also requires one map-level
`Reopening` reservation, exact pre-cut inverse compensation, lock release after durable terminal
fsync but before network publication, and one typed RepoList/scope finalization with an optional
user-selected opaque fallback binding.

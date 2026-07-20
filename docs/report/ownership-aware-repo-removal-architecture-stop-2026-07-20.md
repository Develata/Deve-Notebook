# Ownership-aware RemoveLocalRepo architecture stop — 2026-07-20

> 本报告是 `docs/report/` 下的时点证据，不是 live contract。用户已确认
> `RemoveLocalRepo` 应删除本机 Deve-owned repo state，但保留 Projection Workspace、
> Markdown/附件与 `.git`。当前停在 DB authority retirement、repair trigger 与 replay
> protection 的实现架构裁定；未获 USER 批准前不进入代码改造。

## Metadata

- `Date`: `2026-07-20`
- `Status`: `Awaiting USER architecture decision`
- `Baseline`: `main@73ab0f4fe`
- `Approved product semantics`: delete local canonical Redb, exact workspace `.notegit`,
  locator row, alias row and transient catalog tombstone; preserve workspace root,
  Markdown/attachments, `.git`, remote shadows, explicit exports/backups and unrelated RepoIds
- `Explicitly unchanged`: Ledger envelope/payload, `DEVELDG3`, Redb v4 table schema,
  Projection format, sync facts, Remote Import ownership and current WS F4/v4

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
  -> terminal publication
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

**Recommendation: A1.** It remains inside the already named authority storage boundary. It does
not create a new public crate or move Ledger authority into the lifecycle coordinator.

The cross-process proof must be project-owned and acquired before unlink. Redb/open-file failure
alone is not the protocol. Any inability to obtain exclusive retirement produces typed cleanup
debt and leaves the DB untouched.

## 3. Decision B — committed cleanup recovery

| Route | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| B1 | Hybrid recovery: startup only classifies durable receipts. It may automatically continue a committed settlement when manifest digest, every remaining target fingerprint and containment are unchanged. Any drift becomes `RepairRequired`. Extend the existing repair surface with request-scoped dry-run output; actual cleanup requires explicit `--apply`. Server-held authority uses the same internal coordinator/proxy rather than opening Redb directly. | Exact unchanged crashes self-heal; changed identities never receive implicit destructive authority; operator sees the exact remaining set. | Adds a narrow repair producer and receipt state machine; must prove exclusivity with normal lifecycle jobs. |
| B2 | Never auto-continue. Every post-crash remaining target requires explicit repair apply. | Simplest destructive policy and easiest audit. | Normal crash recovery leaves avoidable long-lived tombstones and manual work even when all identities are unchanged. |
| B3 | Automatically retry every path in the saved manifest until absent. | Maximum apparent availability. | A replaced ancestor/target can receive stale deletion authority; violates fail-closed ownership. |

**Recommendation: B1.** “Restart does not resume the old job” remains true; a separate recovery
owner settles already committed truth. Dry-run is read-only. Apply cannot expand the original
manifest or override fingerprint drift; changed identity requires a new explicit product-level
decision, not a force flag.

## 4. Decision C — destructive request replay protection

| Route | Design | Benefit | Cost / failure mode |
| --- | --- | --- | --- |
| C1 | Retain a minimal host-global removal replay fence keyed by `request_id`, exact RepoId, removed membership revision and terminal outcome digest. It contains no Ledger content, path, alias or workspace inventory and is never used as repo membership. | No WS change; old retry can return the prior terminal outcome and can never target a later incarnation; very small state. | Deliberately retains tiny control-plane metadata after repo state cleanup; needs bounded corruption checks even though it is not pruned. |
| C2 | Add a prepare-issued confirmation token / durable membership incarnation to every remove intent and reject it after that incarnation ends. | Self-contained destructive authorization and no permanent request history. | Changes current F4/v4 wire/product flow and confirmation UX; requires another protocol revision and broader client migration. |
| C3 | Use the current bounded lifecycle receipt only. | No new mechanism. | Unsafe once the receipt is pruned and the RepoId is re-admitted. |

**Recommendation: C1.** This is control-plane idempotency evidence, not retained repo data. It is
the smallest route and avoids a protocol bump. Corruption must fail closed; loss of the fence
blocks ambiguous replay rather than treating it as a new remove.

## 5. Combined recommendation

Approve:

```text
A1 + B1 + C1
```

The resulting ownership direction is:

```text
RepoLifecycleCoordinator
  -> authority_storage_runtime retirement capability
  -> owner-specific exact cleanup commands
  -> durable cleanup receipt / tombstone retirement

UI / WS handler
  -> typed intent and typed outcome only
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
- old request replay cannot affect a re-admitted same RepoId;
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

Pending. Approval of the product-level RemoveLocalRepo semantics does not by itself authorize the
above runtime ownership migration, repair surface or replay-fence persistence. Implementation
must remain stopped until USER explicitly approves a route for Decisions A, B and C.

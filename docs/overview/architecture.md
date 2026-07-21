# Deve-Note Architecture Overview

One-page architecture map with plan-first verification: the blueprint view is derived from `docs/plan/` and related feature/acceptance documents, while the code view is a lagging implementation view that must eventually converge to the plan. Drift status is recorded in [`architecture-diff.md`](./architecture-diff.md).

Primary files: [`architecture-doc.lisp`](./architecture-doc.lisp), [`architecture-code.lisp`](./architecture-code.lisp), [`architecture-diff.md`](./architecture-diff.md), [`architecture.dot`](./architecture.dot), and [`architecture.svg`](./architecture.svg).
The `.dot` and `.lisp` files are assembled from fragments by `scripts/generate-architecture-dot.sh` and `scripts/generate-architecture-lisp.sh`.

## Layered Architecture (4-layer canonical call architecture)

```
┌─────────────────────────────────────────────────────┐
│  Layer 1 — User Operation                            │
│  Type Username · Open Command Palette ·              │
│  Submit Login Form · Choose Document Result · ...    │
└────────────────────┬─────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  Layer 2 — Instruction Interface                     │
│  Form actions · UI callbacks · request senders ·     │
│  command handlers · HTTP handlers · WS handlers      │
└────────────────────┬─────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  Layer 3 — Flow Coordination                         │
│  branch switch flow · doc edit flow ·                │
│  repo file ops flow · merge runtime flow · ...       │
└────────────────────┬─────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  Layer 4 — Execution Domain                          │
│  ledger · sync · source_control · tree ·             │
│  protocol · security · plugin · search               │
└─────────────────────────────────────────────────────┘
```

Each arrow is a "receives / dispatches to" relation. Layer 1 is no longer a list of entry surfaces such as CLI, Palette, or Slash; it is a list of atomic user actions. Surface type belongs to node metadata, not to the layer shape. No layer ever skips downward and no layer ever calls upward.

This architecture follows the governing rule in [`00_engineering_constitution.md`](../plan/00_engineering_constitution.md). It also carries two sidecar dimensions that are not part of the four-layer call cascade:

- `Object Plane`: the concrete objects ultimately touched by flows, such as `doc::content`, `pending_local_edit`, `confirmed_op`, `tree::projection`, or `repo::scope`
- `Ownership Axis`: the long-lived root domains that own modules and execution responsibilities over time

Each layer may contain grouped flows. For Layer 1, every option has three ownership levels: enclosing layer, platform bucket (`web`, `desktop`, `cli`, `android`, etc.), and flow group such as `login`, `session-expired / unauthorized`, or `open-doc`. Reused flows go into explicit intersection buckets such as `shared(web,desktop)`.

The current graph also carries three extra architecture cues:
- group labels include their governing plan chapters
- critical gates such as scope / connection validity may appear as explicit note nodes
- each major flow can be read through a group-level spine in addition to fine-grained node edges

When reading the current generated SVG, treat the visible fourth layer as the current rendering of `execution domains`, not as a competing fifth call layer. The object plane is rendered as a sidecar cluster with dotted edges from execution domains; it is not part of the main downward call cascade.

## Watcher Runtime Ownership Slice

The approved workspace-ingestion path preserves the same four-layer direction while separating lifecycle ownership from read-only admission:

```text
UI / HTTP / WS handlers
  -> typed Prepare/Execute intent / RepoLifecycleJobRuntime
  -> RepoLifecycleCoordinator
  -> LocalAuthorityRuntime + RepoMutationPublicationGate + WatcherRuntimeView
  -> WatcherSupervisor
  -> RepoWatcherHandle
  -> FsWatcherBackend adapter
  -> notify/platform backend
```

- `RepoWatcherHandle` is the non-clone single-repo execution owner in core.
- `WatcherSupervisor` is the CLI host runtime owner of repo slots, generations and lifecycle; handlers never receive it.
- `WatcherRuntimeView` exposes only snapshot/aggregate readiness to `AppState`, mutation admission and `/api/node/role`.
- `RepoLifecycleJobRuntime` is the transport-independent owner of accepted create/remove jobs; handlers may stop waiting but cannot cancel durable convergence.
- `LocalAuthorityRuntime` is the sole per-RepoId Redb owner. Non-clone leases borrow the database; product mutation admission closes before provider quiesce and watcher E2, then Quiescing drains bounded in-flight use. The per-RepoId lock pathname is persistent host coordination identity; its OS handle stays held through exact cleanup/tombstone retirement, and later same-RepoId admission creates a new slot generation.
- Same-RepoId readmission first installs one map-level `Reopening` reservation, performs lock/DB/catalog I/O outside the map mutex, and exact-CAS installs the new Active generation. Before the Removed cut, failure runs inverse compensation for authority, watcher, provider and sealed owner plans; compensation failure stays typed readonly/repair.
- `RepoLifecycleCoordinator` is the only create/remove flow coordinator allowed to request mount transitions. Host-local alias changes never enter this slice. `RemoveLocalRepo` first returns a backend preview and five-minute one-time token, then uses a typed ownership manifest and owner-specific cleanup APIs. It deletes local Redb/Deve runtime while preserving the workspace root, Markdown/attachments, `.git`, remote shadows and operator recovery input.
- `remote_import_runtime` owns its artifact removal plan. Safe non-applied states are warning+owner cleanup; Pending/Degraded/unknown states block rather than granting the lifecycle coordinator path authority. The owner seals cleanup before authority retirement and performs artifact-only cleanup after the Removed cut.
- zero local repos is a valid `NoScope` host state. Watcher expected=0 is healthy; login, diagnostics and Create stay available.
- A fallback is an optional user choice bound during Prepare, never backend auto-selection. Cleanup fsyncs its durable terminal result and releases the authority lock before best-effort publication; clients receive final RepoList and scope in one typed finalization.
- durable `RepoHealth` and process-local `RepoMountState` are orthogonal. Workspace-dependent writes require `Healthy + Mounted`; watcher failure never becomes a projection fault or Ledger fact.
- the Web shell renders typed blocker/health state only. It does not parse failure detail, decide restart policy or perform watcher recovery.

The owned supervisor, exact-slot mounted admission, runtime failure cut, public aggregate health,
E2 final-state shutdown, host-owned lifecycle jobs and the existing prepare/cut/settle skeleton are
implemented. `flow.repo.lifecycle` remains active drift because ownership-aware removal still lacks
safe per-RepoId authority retirement, zero-repo composition, F4/v5 preview-token admission,
manifest-bound owned-state settlement, exact repair and destructive UI evidence. A catalog tombstone
alone is not evidence of physical cleanup.

## Cross-host Data and Host-local Interaction

As a soft design principle, Deve keeps the machine-facing cross-host data plane and the
human-facing local interaction plane maximally independent. Cross-host state prioritizes
precise identity, complete Markdown/Ledger facts, deterministic replay and authenticated
admission. Local labels and visual preferences remain local unless transferring them is
necessary for those guarantees.

Repo naming is the reference case: peers share an immutable RepoId and never exchange the
host's alias. `host_repo_alias_runtime` owns the local display mapping; Projection Locator
owns an immutable physical `workspace_segment`. A user may export/import the small JSON
mapping explicitly, but alias changes never become Ledger/sync facts and never move the
workspace. This is a preference for low coupling, not a license to discard mechanisms that
materially improve correctness, safety or usability.

## Remote Import Ownership Slice

```text
Remote provider
  -> remote_projection_transport_runtime
  -> immutable manifest/blob capture
  -> remote_import_runtime
  -> typed review/blockers
  -> sealed source-specific authority writer
  -> Ledger commit
  -> Projection writeback
  -> Workspace
```

Remote Projection owns push/source streaming only. Remote Import owns durable session/candidate lifecycle and its typed repo-removal artifact plan, but cannot write authority tables directly. Source Control and External Changes are sibling domains, not import controllers; the Web client is a thin typed projection. B4 已激活 backend/CLI/product wire并删除旧 pull substitute；82-flow modeled slice 同时诚实登记 Remote Import client 与 ownership-aware lifecycle drift。

Post-commit Projection outcome uses one repo-local Redb v4 settlement boundary:
`Pending -> Written` updates only the stored receipt, while `Pending -> Degraded`
atomically writes typed `PROJECTION_FAULTS` recovery evidence and the receipt CAS.
The side table is owned by projection persistence, is not a Ledger Fact, and is
never synchronized; the retired host-wide TOML journal is not a fallback.

## Artifact Roles

- `architecture-doc.lisp`: doc-derived operation-first blueprint view, emitted from ordered doc fragments.
- `architecture-code.lisp`: hand-curated implementation view, emitted from ordered code fragments.
- `architecture.dot` / `architecture.svg`: generated visual graph for the four-layer canonical call architecture.
- `architecture-diff.md`: operation-level comparison and drift registry.

## Lisp Conventions

Both `.lisp` files use keyword-style s-expressions:

```lisp
(system :name deve-note :version "0.0.1" :layers (user-operation application module core))

(user-operation :id op.auth.login.submit :label "Submit Login Form" :kind submit :calls (app.auth.login.submit))
(application :id app.auth.login.submit :label "login::submit" :kind form-submit :calls (mod_sec_auth mod_sec_jwt mod_proto_auth))
(module :id mod_sec_auth :label "security::auth" :parent core.security :code "crates/core/src/security/auth/")
(core :id core.sync :label "sync" :kind runtime :code "crates/core/src/sync/")
```

The current generated artifacts keep the stable internal IDs `application`, `module`, and `core`, but their canonical meanings are now:

- `application` = instruction interface
- `module` = flow coordination
- `core` = execution domain

- Fields use `:keyword` prefix — AI parsers and humans can both read them without positional guessing.
- `:calls (x y z)` lists downstream targets — always pointing toward a lower layer in the canonical call cascade.
- Layer 1 node IDs should use `op.<domain>.<flow>.<verb>` naming.
- `group` records define layer-internal bundles such as `login`, `session-expired / unauthorized`, `command-palette`, `branch-switch`, `repo-switch`, `stage / unstage`, `source-control commit`, `native ai-chat`, or `open-doc`.
- `:chapter 03_storage/watcher#watcher-contract` references a stable plan anchor; `:code "path/to/file.rs"` references authoritative code.

`Object Plane` is modeled with sidecar `object-plane` / `object` forms in the generated Lisp. `Ownership Axis` remains sidecar semantics expressed through module/core ownership metadata rather than a canonical call layer.

## Authority Rule

`docs/plan/` is the authoritative source.

- Plan and code must ultimately form a strict bijection.
- Feature and operation documents refine plan-visible behavior, but must not override plan contracts.
- When code lags behind the plan, the architecture view must preserve the plan shape and mark code as divergent.
- "Existing in code" never justifies changing the blueprint by itself; the default fix is to align code to plan, not the reverse.

## The `*` Convention (Divergence Marker)

A node gains a `*` marker when **plan-side blueprint and code-side implementation don't agree**.

`architecture-diff.md` is the source for current drift state. When future drift appears, update that registry and regenerate the graph.

When divergence markers are enabled:

- **In code, not in plan** → code has a command, module, or route that the plan does not describe.
- **In plan, not in code** → plan promises behavior that has no corresponding implementation yet.
- **Both exist but with different shape** → e.g. plan says one response flow, code has a different handler/module split.

**Current divergences**: see [`architecture-diff.md`](./architecture-diff.md).

## Regenerating This View

The fragment sources are hand-curated. The automation target is:

1. `architecture-doc.lisp` — generated from `docs/plan/*.md` plus `docs/features/operations/*.md`, with plan as the authority source.
2. `architecture-code.lisp` — generated by tracing implementation flows across `apps/web/src/` user actions and callbacks, `apps/cli/src/server/` handlers, and leaf modules in `crates/core/src/`.
3. `architecture-diff.md` — generated by an operation-level diff tool that walks both trees and flags mismatches by flow, response split, or module/core mapping.
4. `architecture.svg` — run `dot -Tsvg architecture.dot -o architecture.svg`.
5. `scripts/check-architecture-registry.sh` — verify flow count, drift-map, operation files, Lisp IDs, and spine coverage stay in sync.

Until the generators become semantic extractors, treat the fragment sources as **hand-curated views**. Update the plan-side fragments first when the blueprint changes, then update the code-side fragments only to reflect actual implementation progress against that blueprint.

## Related Documents

- [`docs/coverage-matrix.md`](../coverage-matrix.md) — three-layer chapter mapping (plan ↔ features ↔ acceptance-cases)
- [`docs/features/operation-coverage.md`](../features/operation-coverage.md) — operation-to-acceptance coverage registry
- [`docs/plan/deve-note plan.md`](../plan/deve-note%20plan.md) — engineering blueprint index
- [`docs/plan/AGENTS.md`](../plan/AGENTS.md) — plan-code bijection enforcement rules

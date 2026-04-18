# Deve-Note Architecture Overview

One-page architecture map with plan-first verification: the blueprint view is derived from `docs/plan/` and related feature/acceptance documents, while the code view is a lagging implementation view that must eventually converge to the plan. The current modeled slice is aligned across plan and code, with no active drift markers.

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

## Current Refactor Status

- `architecture-doc.lisp` has been refactored to an operation-first baseline and is now emitted from ordered doc fragments.
- `architecture-code.lisp` has also been uplifted to the same operation-first layer model and is now emitted from ordered code fragments, though it is still a hand-curated implementation baseline rather than a generated truth source.
- `architecture.dot` should now be read as the plan-side blueprint graph for the four-layer canonical call architecture.
- The currently modeled high-value flows are `login`, `session-expired / unauthorized`, `command-palette`, `repo-scoped sync handshake`, `repo-scoped key exchange`, `repo-scoped sync transfer`, `branch-switch`, `repo-switch`, `stage / unstage`, `discard file`, `discard pending`, `resolve conflict`, `source-control commit`, `history / commit diff`, `commit-and-push`, `merge peer`, `merge runtime`, `native ai-chat`, `trusted external agent boundary`, `plugin-host / plugin-call boundary`, `search/query`, `repo file operations`, `document edit / confirmed op`, `leave document / pending edit guard`, `open-doc`, `release / CI`, `CLI control commands`, `CLI vault indexing`, `CLI server runtime`, `CLI export / inspect`, `CLI repair / admin`, `settings update`, `settings env defaults`, `settings file config`, `settings UI preferences`, `settings runtime feedback`, `rendering cursor reveal`, `rendering math / mermaid`, `i18n locale / error`, and `tech-stack runtime budget`.
- `architecture-diff.md` has been rebuilt into an operation-level comparison pass and currently records no active structural mismatch inside the modeled slice.
- The current SVG should be read as the shared baseline; future explicit drift markers are still driven by `architecture-diff.md`.

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
- `:chapter 04_storage#watcher-contract` references a stable plan anchor; `:code "path/to/file.rs"` references authoritative code.

`Object Plane` is modeled with sidecar `object-plane` / `object` forms in the generated Lisp. `Ownership Axis` remains sidecar semantics expressed through module/core ownership metadata rather than a canonical call layer.

## Authority Rule

`docs/plan/` is the authoritative source.

- Plan and code must ultimately form a strict bijection.
- Feature and operation documents refine plan-visible behavior, but must not override plan contracts.
- When code lags behind the plan, the architecture view must preserve the plan shape and mark code as divergent.
- "Existing in code" never justifies changing the blueprint by itself; the default fix is to align code to plan, not the reverse.

## The `*` Convention (Divergence Marker)

A node gains a `*` marker when **plan-side blueprint and code-side implementation don't agree**.

At this moment, the plan-side and code-side Lisp views both use the new layer semantics, and the diff report has been rebuilt at the operation level. The currently modeled slice has no active drift. If future drift appears, update the registry in [`architecture-diff.md`](./architecture-diff.md) and regenerate the graph.

When divergence markers are enabled:

- **In code, not in plan** → code has a command, module, or route that the plan does not describe.
- **In plan, not in code** → plan promises behavior that has no corresponding implementation yet.
- **Both exist but with different shape** → e.g. plan says one response flow, code has a different handler/module split.

**Current divergences**: see [`architecture-diff.md`](./architecture-diff.md). For the currently modeled operation slice, there are no active mismatches.

## Regenerating This View

This baseline was built by hand. The intent is for future iterations to automate:

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

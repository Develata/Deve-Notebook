# Deve-Note Architecture Overview

One-page architecture map with plan-first verification: the blueprint view is derived from `docs/plan/` and related feature/acceptance documents, while the code view is a lagging implementation view that must eventually converge to the plan. The currently modeled operation slice is aligned, so the SVG is presently shown as a clean baseline without `*` markers.

**Files in this directory**:

| File | Purpose |
|---|---|
| [`architecture.md`](./architecture.md) | This file — human entry point |
| [`architecture-doc.lisp`](./architecture-doc.lisp) | Architecture view derived from `docs/plan/`, `docs/features/operations/`, and `docs/acceptance-cases/` |
| [`architecture-code.lisp`](./architecture-code.lisp) | Architecture view derived from actual implementation flows in `apps/web/src/`, `apps/cli/src/server/`, and `crates/core/src/` |
| [`architecture-diff.md`](./architecture-diff.md) | Comparison report for the current operation slice; future `*` markers should trace back here |
| [`architecture.dot`](./architecture.dot) | Generated Graphviz source for the SVG diagram |
| [`architecture.svg`](./architecture.svg) | Rendered SVG (run `dot -Tsvg architecture.dot -o architecture.svg`) |

The `.dot` file is now assembled from `docs/overview/graph/fragments/*.dotfrag` by `scripts/generate-architecture-dot.sh`.

## Layered Architecture (4 layers, operation-first)

```
┌─────────────────────────────────────────────────────┐
│  Layer 1 — User Operation                            │
│  Type Username · Open Command Palette ·              │
│  Submit Login Form · Choose Document Result · ...    │
└────────────────────┬─────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  Layer 2 — Application Response                      │
│  Form actions · UI callbacks · request senders ·     │
│  HTTP handlers · WS handlers                         │
└────────────────────┬─────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  Layer 3 — Module (finest leaves)                    │
│  sync::watcher · source_control::pending_fs ·        │
│  ledger::manager · tree::{manager,ops,node} · ...    │
└────────────────────┬─────────────────────────────────┘
                     ↓
┌─────────────────────────────────────────────────────┐
│  Layer 4 — Core Subsystems                           │
│  ledger · sync · source_control · tree ·             │
│  protocol · security · plugin · search               │
└─────────────────────────────────────────────────────┘
```

Each arrow is a "receives / dispatches to" relation. Layer 1 is no longer a list of entry surfaces such as CLI, Palette, or Slash; it is a list of atomic user actions. Surface type belongs to node metadata, not to the layer shape. No layer ever skips downward and no layer ever calls upward.

Each layer may also contain multiple grouped domains or flows. For example, the login-related user operations should appear together as a `login` group inside Layer 1 rather than being flattened across the whole layer.
For Layer 1 specifically, each option now has three ownership levels: the enclosing layer, the platform bucket (`web`, `desktop`, `cli`, `android`, etc.), and then the flow/module group such as `login`, `session-expired / unauthorized`, or `open-doc`.
When one option flow is reused across multiple platforms, the graph places it in an explicit intersection bucket such as `shared(web,desktop)` instead of forcing it into only one platform frame.

The current graph also carries three extra architecture cues:
- group labels include their governing plan chapters
- critical gates such as scope / connection validity may appear as explicit note nodes
- each major flow can be read through a group-level spine in addition to fine-grained node edges

## Current Refactor Status

- `architecture-doc.lisp` has been refactored to an operation-first baseline.
- `architecture-code.lisp` has also been uplifted to the same operation-first layer model, but it is still a hand-curated implementation baseline rather than a generated truth source.
- `architecture.dot` should now be read as the plan-side blueprint graph for that new layer model.
- The currently modeled high-value flows are `login`, `session-expired / unauthorized`, `command-palette`, `repo-scoped sync handshake`, `repo-scoped key exchange`, `repo-scoped sync transfer`, `branch-switch`, `repo-switch`, `stage / unstage`, `discard file`, `discard pending`, `resolve conflict`, `source-control commit`, `history / commit diff`, `commit-and-push`, `merge peer`, `merge runtime`, `native ai-chat`, and `open-doc`.
- `architecture-diff.md` has been rebuilt into an operation-level comparison pass and currently reports no unresolved structural mismatch inside the modeled slice.
- The current SVG should be read as a clean baseline for the modeled slice, not as a starred divergence map.

## How To Read The Lisp Files

Both `.lisp` files use **keyword-style s-expression** (Common Lisp / Emacs Lisp style):

```lisp
(system :name deve-note :version "0.0.1" :layers (user-operation application module core))

(layer :id user-operation :order 1 :description "...")

(user-operation :id op.auth.login.submit :label "Submit Login Form" :kind submit :calls (app.auth.login.submit))
(application :id app.auth.login.submit :label "login::submit" :kind form-submit :calls (mod_sec_auth mod_sec_jwt mod_proto_auth))
(module :id mod_sec_auth :label "security::auth" :parent core.security :code "crates/core/src/security/auth/")
(core :id core.sync :label "sync" :kind runtime :code "crates/core/src/sync/")
```

**Key conventions**:
- Fields use `:keyword` prefix — AI parsers and humans can both read them without positional guessing.
- `:calls (x y z)` lists downstream targets — always pointing toward a lower layer.
- Layer 1 node IDs should use `op.<domain>.<flow>.<verb>` naming.
- `group` records define layer-internal bundles such as `login`, `session-expired / unauthorized`, `command-palette`, `branch-switch`, `repo-switch`, `stage / unstage`, `source-control commit`, `native ai-chat`, or `open-doc`.
- `:chapter 04_storage#watcher-contract` references a stable anchor declared in a plan chapter via `{#watcher-contract}`.
- `:code "path/to/file.rs"` references the authoritative code location.
- IDs are dotted, e.g. `core.sync.watcher`, to preserve hierarchy without repeating the parent.

## Authority Rule

`docs/plan/` is the authoritative source.

- Plan and code must ultimately form a strict bijection.
- Feature and operation documents refine plan-visible behavior, but must not override plan contracts.
- When code lags behind the plan, the architecture view must preserve the plan shape and mark code as divergent.
- "Existing in code" never justifies changing the blueprint by itself; the default fix is to align code to plan, not the reverse.

## The `*` Convention (Divergence Marker)

A node gains a `*` marker when **plan-side blueprint and code-side implementation don't agree**.

At this moment, the plan-side and code-side Lisp views both use the new layer semantics, and the diff report has been rebuilt at the operation level. The currently modeled slice has no unresolved structural mismatch, so the SVG is intentionally rendered without `*` markers.

When divergence markers are enabled:

- **In code, not in plan** → code has a command, module, or route that the plan does not describe.
- **In plan, not in code** → plan promises behavior that has no corresponding implementation yet.
- **Both exist but with different shape** → e.g. plan says one response flow, code has a different handler/module split.

**Current divergences**: see [`architecture-diff.md`](./architecture-diff.md). For the currently modeled operation slice, there is no remaining unresolved structure mismatch.

## Regenerating This View

This baseline was built by hand. The intent is for future iterations to automate:

1. `architecture-doc.lisp` — generated from `docs/plan/*.md` plus `docs/features/operations/*.md`, with plan as the authority source.
2. `architecture-code.lisp` — generated by tracing implementation flows across `apps/web/src/` user actions and callbacks, `apps/cli/src/server/` handlers, and leaf modules in `crates/core/src/`.
3. `architecture-diff.md` — generated by an operation-level diff tool that walks both trees and flags mismatches by flow, response split, or module/core mapping.
4. `architecture.svg` — run `dot -Tsvg architecture.dot -o architecture.svg`.

Until the generators exist, treat the `.lisp` files as **hand-curated views**. Update the plan-side view first when the blueprint changes, then update the code-side view only to reflect actual implementation progress against that blueprint.

## Related Documents

- [`docs/coverage-matrix.md`](../coverage-matrix.md) — three-layer chapter mapping (plan ↔ features ↔ acceptance-cases)
- [`docs/plan/deve-note plan.md`](../plan/deve-note%20plan.md) — engineering blueprint index
- [`docs/plan/AGENTS.md`](../plan/AGENTS.md) — plan-code bijection enforcement rules

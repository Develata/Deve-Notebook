# Architecture Diff Report (doc vs code)

Generated: 2026-04-10 (manual operation-first pass)

This report compares [`architecture-doc.lisp`](./architecture-doc.lisp)
against [`architecture-code.lisp`](./architecture-code.lisp) using the
shared four-layer model:

- `user-operation`
- `application`
- `module`
- `core`

Plan remains the authority source. The current comparison is now limited
to the modeled operation slice rather than the older route/CLI inventory
view.

## Summary

| Area | Status | Notes |
|---|---|---|
| Flow set | aligned | the same 19 high-value flows exist on both sides |
| User operations | aligned | current IDs and flow grouping match |
| Application responses | aligned | current modeled response nodes match |
| Module/core ownership | aligned | current core ownership and tree leaf naming match |
| Scope hygiene | aligned | legacy inventory has been removed from the doc-side slice |

## 1. Flows Already Close To Bijection

These flows are now structurally close across all four layers:

- `login`
- `session-expired / unauthorized`
- `command-palette`
- `repo-scoped sync handshake`
- `repo-scoped key exchange`
- `repo-scoped sync transfer`
- `branch-switch`
- `repo-switch`
- `stage / unstage`
- `discard file`
- `discard pending`
- `resolve conflict`
- `source-control commit`
- `history / commit diff`
- `commit-and-push`
- `merge peer`
- `merge runtime`
- `native ai-chat`
- `open-doc`

For these flows, the user-operation IDs, application groups, and
core-domain ownership are now effectively aligned between plan and code.

## 2. Current State

Within the currently modeled operation slice, the plan-side and code-side
views are now structurally aligned across:

- flow set
- user-operation IDs
- application grouping
- module ownership
- core ownership

This does not mean the whole system is finished. It means the current
operation-first slice no longer has an unresolved mismatch recorded in
this report.

## 3. What To Do Next

1. Regenerate `*` markers into the SVG from this operation-level diff
   instead of from the old inventory report.
2. Expand the shared slice with additional flows only if they can be
   added to both plan-side and code-side views together.
3. If a broader CLI/admin inventory view is still wanted, keep it as a
   separate report rather than mixing it back into this operation slice.

For the currently modeled slice, plan and code now read as a practical
bijective baseline.

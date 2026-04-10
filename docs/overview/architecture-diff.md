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
| Flow set | aligned | the same 21 high-value flows exist on both sides |
| User operations | aligned | current IDs and flow grouping match |
| Application responses | drift | `trusted external agent boundary` still differs between plan and code |
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
- `plugin-host / plugin-call boundary`
- `open-doc`

For these flows, the user-operation IDs, application groups, and
core-domain ownership are now effectively aligned between plan and code.

## 2. Known Divergence: `trusted external agent boundary`

The newly modeled `trusted external agent boundary` flow is intentionally
not yet bijective at the application layer.

Plan-side blueprint requires:

- `trusted-cli` to remain default-off
- `ai.agent_bridge.enabled = true`
- `ai.agent_bridge.trusted = true`
- `AGENT_CLI_PATH` explicitly set
- failure to satisfy any gate must fall back to `native` and show a
  clear disabled reason

Current code-side implementation does not yet match that shape:

- [settings_sections.rs](/home/develata/gitclone/Deve-Notebook/apps/web/src/components/settings_sections.rs)
  exposes the `trusted-cli` switch directly
- [extensions.rs](/home/develata/gitclone/Deve-Notebook/apps/web/src/components/sidebar/extensions.rs)
  also exposes `agent-bridge` as a selectable channel without a trust gate
- [agent_bridge.rs](/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/agent_bridge.rs)
  reads `AGENT_CLI_PATH` or falls back to `opencode`, then always attempts
  to spawn the CLI

So the mismatch is not in flow presence or core ownership. It is in
application-layer contract:

- plan says `guard -> enable or fallback`
- code currently does `select -> spawn`

## 3. Current State

Within the currently modeled operation slice:

- flow set is aligned
- user-operation IDs are aligned
- module ownership is aligned
- core ownership is aligned
- one application-layer mismatch remains in `trusted external agent boundary`

This means the slice is no longer a fully clean bijective baseline, but
it is still narrow and explicit.

## 4. What To Do Next

1. Implement the `trusted-cli` visibility and fallback gate so code
   matches `enabled + trusted + AGENT_CLI_PATH`.
2. Regenerate `*` markers into the SVG from this operation-level diff
   instead of from the old inventory report.
3. Expand the shared slice with additional flows only if they can be
   added to both plan-side and code-side views together.

For the currently modeled slice, every flow except `trusted external
agent boundary` reads as a practical bijective baseline.

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
| Application responses | aligned | current response taxonomy now matches across the modeled slice |
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

## 2. Current Alignment Notes

The previously tracked `trusted external agent boundary` mismatch is now
closed at the application layer.

Plan-side blueprint required:

- `trusted-cli` default-off
- `ai.agent_bridge.enabled = true`
- `ai.agent_bridge.trusted = true`
- `AGENT_CLI_PATH` explicitly set
- failure to satisfy any gate must fail closed and surface a clear
  disabled reason

Current code-side implementation now matches that contract:

- [settings_sections.rs](/home/develata/gitclone/Deve-Notebook/apps/web/src/components/settings_sections.rs)
  reads backend capabilities and disables the `trusted-cli` choice when
  gates are not satisfied
- [extensions_channels.rs](/home/develata/gitclone/Deve-Notebook/apps/web/src/components/sidebar/extensions_channels.rs)
  renders the channel as unavailable with an explicit reason
- [agent_bridge.rs](/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/agent_bridge.rs)
  now fail-closes on policy checks instead of defaulting to `opencode`
- [policy.rs](/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/agent_bridge/policy.rs)
  centralizes the `enabled + trusted + AGENT_CLI_PATH` gate

## 3. Current State

Within the currently modeled operation slice:

- flow set is aligned
- user-operation IDs are aligned
- application responses are aligned
- module ownership is aligned
- core ownership is aligned

This means the slice is once again a practical clean bijective baseline.

## 4. What To Do Next

1. Regenerate `*` markers into the SVG from this operation-level diff
   instead of from the old inventory report if unresolved drift returns.
2. Expand the shared slice with additional flows only if they can be
   added to both plan-side and code-side views together.
3. Keep `trusted external agent boundary` under this same contract if
   backend selection or agent spawn behavior changes again.

For the currently modeled slice, all 21 high-value flows now read as a
practical bijective baseline.

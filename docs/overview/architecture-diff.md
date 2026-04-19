# Architecture Diff Report (doc vs code)

Generated: 2026-04-18 (manual operation-first pass)

This report compares [`architecture-doc.lisp`](./architecture-doc.lisp)
against [`architecture-code.lisp`](./architecture-code.lisp). Plan remains
the authority source, and the comparison is limited to the modeled
operation slice rather than the older route/CLI inventory view.

## Modeled Slice

Keep this block stable. The graph generator reads the drift registry below.

<!-- modeled-slice:start -->
- Flow count: `47`
- Status: `aligned`
- Active drift count: `0`
<!-- modeled-slice:end -->

## Summary

| Area | Status | Notes |
|---|---|---|
| Flow set | aligned | the same 47 high-value flows exist on both sides |
| User operations | aligned | current IDs and flow grouping match |
| Instruction interfaces | aligned | response taxonomy matches across the modeled slice |
| Coordination/execution mapping | aligned | release / CI now treats `release.yml` as the only required workflow surface |
| Scope hygiene | aligned | legacy inventory is outside this slice |

## Drift Registry

Use one entry per divergent flow. Labels must match the flow registry.
<!-- drift-registry:start -->
- `none`
<!-- drift-registry:end -->

## Flow Registry

Use this registry as the stable label set for the diff and SVG marker map.

<!-- flow-registry:start -->
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
- `trusted external agent boundary`
- `plugin-host / plugin-call boundary`
- `search/query`
- `repo file operations`
- `document edit / confirmed op`
- `leave document / pending edit guard`
- `open-doc`
- `release / CI`
- `CLI control commands`
- `CLI vault indexing`
- `CLI server runtime`
- `CLI export / inspect`
- `CLI repair / admin`
- `settings update`
- `settings env defaults`
- `settings file config`
- `settings UI preferences`
- `settings runtime feedback`
- `rendering cursor reveal`
- `rendering math / mermaid`
- `i18n locale / error`
- `i18n locale selection`
- `i18n error mapping`
- `i18n localized formatting`
- `i18n hardcoded audit`
- `tech-stack runtime budget`
- `tech-stack dependency policy`
- `tech-stack runtime budget check`
- `tech-stack platform / release channel`
<!-- flow-registry:end -->

## Current Alignment Notes

The previously tracked `trusted external agent boundary` mismatch is now
closed at the application layer. Code matches the plan contract:

- `trusted-cli` stays default-off
- `ai.agent_bridge.enabled = true`
- `ai.agent_bridge.trusted = true`
- `AGENT_CLI_PATH` must be explicitly set
- failed gates fail closed and surface a clear disabled reason

The closing implementation is represented by:

- [settings_sections.rs](/home/develata/gitclone/Deve-Notebook/apps/web/src/components/settings_sections.rs)
- [extensions_channels.rs](/home/develata/gitclone/Deve-Notebook/apps/web/src/components/sidebar/extensions_channels.rs)
- [agent_bridge.rs](/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/agent_bridge.rs)
- [policy.rs](/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/agent_bridge/policy.rs)

The previous `release / CI` drift is closed. The plan and `.github`
metadata now treat `.github/workflows/release.yml` as the only required
release workflow surface. `nightly.yml` and `speckit-sync-check.yml` are
intentionally outside the current baseline.

## Current State

Within the currently modeled operation slice:

- flow set is aligned
- user-operation IDs are aligned
- instruction interfaces are aligned
- no active coordination ownership gaps remain
- execution-domain ownership is aligned

The slice is a practical bijective baseline with no active drift markers.

## Maintenance Rules

1. Add a flow to both `Flow Registry` and
   [`drift-map.tsv`](./graph/drift-map.tsv) before it can receive a marker.
2. Add active drift to `Drift Registry` only when the modeled flow stops
   matching across plan and code.
3. Regenerate the graph with `scripts/generate-architecture-dot.sh` after
   any registry change.
4. Expand the shared slice only when the new flow can be represented on
   both plan-side and code-side views.

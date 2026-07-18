# Architecture Diff Report (doc vs code)

Generated: 2026-07-18 (Route B B3 sealed-writer pass)

This report compares [`architecture-doc.lisp`](./architecture-doc.lisp)
against [`architecture-code.lisp`](./architecture-code.lisp). Plan remains
the authority source, and the comparison is limited to the modeled
operation slice rather than the older route/CLI inventory view.

## Modeled Slice

Keep this block stable. The graph generator reads the drift registry below.

<!-- modeled-slice:start -->
- Flow count: `79`
- Status: `drifted`
- Active drift count: `4`
<!-- modeled-slice:end -->

## Summary

| Area | Status | Notes |
|---|---|---|
| Flow set | drifted | 79 approved flow labels exist on both sides; four Remote Import flows intentionally map current substitutes/missing carriers |
| User operations | drifted | existing 74 IDs and Remote Projection push align; four Remote Import target flows remain incomplete |
| Instruction interfaces | aligned | response taxonomy matches across the modeled slice |
| Coordination/execution mapping | drifted | Shared transport, immutable session and sealed core Apply are present; product Prepare/Apply/writeback and independent client remain scheduled B4–B5 |
| Scope hygiene | aligned | legacy inventory is outside this slice |

## Drift Registry

Use one entry per divergent flow. Labels must match the flow registry.
<!-- drift-registry:start -->
- `remote import prepare`
- `remote import review`
- `remote import apply`
- `remote import manage`
<!-- drift-registry:end -->

Active drift facts:

1. `remote import prepare`: B1 immutable store and B2 ordered source acquisition exist, but the current product pull still routes through the isolated workspace/External Changes transition; B4 replaces it.
2. `remote import review`: current Source Control/External Changes surfaces substitute for a missing independent runtime/client; B4/B5 remove that routing.
3. `remote import apply`: B3 source-specific sealed whole-session Ledger transaction and ADR 0012 repo-local fault/receipt settlement primitives exist and preserve External Apply semantics；startup fail-closed health now recognizes Pending receipts, while product Mounted admission, Ledger-to-Projection rematerialization orchestration and the Remote Import surface remain B4–B5 work.
4. `remote import manage`: backend Refresh/Discard/dry-run repair/retention exist, but the product lifecycle and cleanup apply path remain B4/W7 work.

## Flow Registry

Use this registry as the stable label set for the diff and SVG marker map.

<!-- flow-registry:start -->
- `login`
- `session-expired / unauthorized`
- `command-palette`
- `context action routing`
- `command surface mode routing`
- `command surface action routing`
- `repo-scoped sync handshake`
- `repo-scoped key exchange`
- `repo-scoped sync transfer`
- `branch-switch`
- `repo file-op shell routing`
- `repo-switch`
- `external changes`
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
- `remote projection push`
- `remote import prepare`
- `remote import review`
- `remote import apply`
- `remote import manage`
- `search/query`
- `repo file operations`
- `document edit / confirmed op`
- `leave document / pending edit guard`
- `open-doc`
- `release / CI`
- `release tag dispatch`
- `release quality gates`
- `release artifact publish`
- `release delivery verification`
- `CLI control commands`
- `CLI parse command`
- `CLI help surface`
- `CLI empty-command guidance`
- `CLI runtime handoff`
- `CLI projection workspace indexing`
- `CLI server runtime`
- `CLI export / inspect`
- `CLI repair / admin`
- `settings update`
- `settings surface open`
- `settings env defaults`
- `settings file config`
- `settings persistence / apply`
- `settings value mutation`
- `settings feedback / render`
- `settings UI preferences`
- `settings runtime feedback`
- `rendering cursor reveal`
- `rendering checkbox writeback`
- `rendering math / mermaid`
- `rendering inline source reveal`
- `rendering link activation gate`
- `rendering large-doc prefetch`
- `rendering large-doc search gate`
- `rendering projection refresh`
- `rendering math source projection`
- `rendering mermaid source projection`
- `rendering outline navigation`
- `i18n locale / error`
- `locale surface switch`
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

- [settings_sections.rs](apps/web/src/components/settings_sections.rs)
- [extensions_channels.rs](apps/web/src/components/sidebar/extensions_channels.rs)
- [agent_bridge/mod.rs](apps/cli/src/server/agent_bridge/mod.rs)
- [policy.rs](apps/cli/src/server/agent_bridge/policy.rs)

The previous `release / CI` drift is closed. The plan and `.github`
metadata now treat `.github/workflows/release.yml` as the only required
release workflow surface. `nightly.yml` and `speckit-sync-check.yml` are
intentionally outside the current baseline.

The Source Control read path is aligned for the current modeled slice:

- browser read requests are gated by repo scope and `scope_nonce`
- `ChangesList`, `CommitHistory`, `DocDiff`, and `CommitDiffResult`
  dispatch reject stale repo, branch, or `scope_nonce` without mutating
  active UI state
- remote `DocDiff` and `CommitDiffResult` handlers cover shadow ledger
  projection success paths
- remote `DocDiff` target-missing and identity-mismatch failures return
  structured `ProtocolError` with browser `scope_nonce`

The WebSocket route scope gate is aligned for the modeled browser routes:

- `core_scoped`, `docs`, `merge`, and `source_control` routes validate
  browser `scope_nonce` before dispatching to handlers
- missing and stale scope nonce paths return structured `ProtocolError`
  with the browser response scope
- route-level tests cover every currently routed scoped input in those
  four layers

The degraded local projection write gate is aligned:

- docs create, document edit, RegisterWriter, source-control mutations,
  merge mutations, and HTTP source-control mutations reject degraded local
  projection before mutating ledger, pending state, staged state, or workspace
- read-only degraded fallback remains a recovery/read surface, not a normal
  mounted write path

## Current State

Within the currently modeled operation slice:

- 74 pre-existing flows remain aligned
- Remote Projection push is aligned; four approved Remote Import flows remain honest current substitute/missing mappings
- Redb v4 and the crate-internal B3 sealed writer are aligned; WS v2 current → v3 target and B4-B6 product/evidence gaps remain release blockers
- no drift is hidden as compatibility support or document-only runtime evidence

The slice is bijective at the registry/label level and intentionally carries four active drift markers until B4–B5 close the product cutover and client flows.

## Maintenance Rules

1. Add a flow to both `Flow Registry` and
   [`drift-map.tsv`](./graph/drift-map.tsv) before it can receive a marker.
2. Add active drift to `Drift Registry` only when the modeled flow stops
   matching across plan and code.
3. Regenerate the graph with `scripts/generate-architecture-dot.sh` after
   any registry change.
4. A planned flow may enter both views only when the code-side view names its real substitute/missing carrier and an active drift marker; never invent a future path.

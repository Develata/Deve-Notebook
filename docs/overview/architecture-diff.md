# Architecture Diff Report (doc vs code)

Generated: 2026-07-21 (R3 preview-token admission landed; removal settlement remains drifted)

This report compares [`architecture-doc.lisp`](./architecture-doc.lisp)
against [`architecture-code.lisp`](./architecture-code.lisp). Plan remains
the authority source, and the comparison is limited to the modeled
operation slice rather than the older route/CLI inventory view.

## Modeled Slice

Keep this block stable. The graph generator reads the drift registry below.

<!-- modeled-slice:start -->
- Flow count: `82`
- Status: `drifted`
- Active drift count: `5`
<!-- modeled-slice:end -->

## Summary

| Area | Status | Notes |
|---|---|---|
| Flow set | drifted | 82 approved flow labels exist on both sides；四个 Remote Import flow保留 B5 client gap，`repo lifecycle` 保留 ownership-aware removal gap |
| User operations | drifted | current F4/v5 Repo Control已删除direct submit-remove并接入Prepare/Execute admission。zero-repo与首个Create配置已收敛；destructive settlement/repair、single typed finalization与Remote Import client尚未收敛 |
| Instruction interfaces | aligned | response taxonomy matches across the modeled slice |
| Coordination/execution mapping | drifted | Shared transport、immutable session、typed review、Mounted sealed Apply、post-commit writeback、repo catalog cut、per-RepoId DB owner/lease与zero-repo composition已存在；owned-state settlement/repair与Remote Import independent client尚未收敛 |
| Scope hygiene | aligned | legacy inventory is outside this slice |

## Drift Registry

Use one entry per divergent flow. Labels must match the flow registry.
<!-- drift-registry:start -->
- `remote import prepare`
- `remote import review`
- `remote import apply`
- `remote import manage`
- `repo lifecycle`
<!-- drift-registry:end -->

Active drift facts:

1. `remote import prepare`: B4 provider-bound backend/CLI Prepare 已替换旧 pull carrier；B5 尚未提供 Web Prepare intent surface。
2. `remote import review`: B4 backend/CLI List/Show/Page/Diff 与 blocker projection 已独立于 Source Control/External Changes；B5 尚未提供 `remote_import_client`。
3. `remote import apply`: B4 已激活 Mounted admission、sealed whole-session Apply、exactly-once receipt 与 Ledger-to-Projection rematerialization；B5 尚未提供 thin Web Apply surface。
4. `remote import manage`: B4 已激活 Refresh/Discard/dry-run repair/explicit cleanup product API；W7 provider quiesce/membership coordination 已接入 host lifecycle，B5 尚未提供 thin Web management surface。
5. `repo lifecycle`: host-owned jobs、session-scoped publication、R1 per-RepoId authority owner/non-clone lease、R2 zero-repo `NoScope`/configured first Create、R3 F4/v5 exact manifest/issuer-bound preview token/atomic `ExecuteAdmitted`，以及R4 O1-FREEZE、manifest-bound quarantine、cut recovery与two-phase terminal settlement均已落地。Option A two-stage owner-prepared same-RepoId reincarnation及唯一production coordinator路径已实现；explicit drift repair、single typed finalization、R5 UI/CLI与R6 fresh跨平台证据仍未实现。

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
- `repo alias set`
- `repo alias transfer`
- `repo lifecycle`
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

- 73 pre-existing flows remain aligned; `repo lifecycle` is explicitly drifted
- Remote Projection push and B4 Remote Import backend/CLI/product wire are aligned; four approved flows retain only the honest B5 client mapping gap
- Redb v4、sealed writer、Mounted admission、post-commit writeback、current F4/v5 Repo Control admission 与 immutable locator are implemented；ownership-aware destructive settlement/repair、B5/B6 与 first-tag freshness evidence remain release blockers
- no drift is hidden as compatibility support or document-only runtime evidence

The slice is bijective at the registry/label level and intentionally carries five active drift markers. B5 closes four independent client gaps; ownership-aware lifecycle requires its own implementation and evidence before B6 can seal 0-drift evidence.

## Maintenance Rules

1. Add a flow to both `Flow Registry` and
   [`drift-map.tsv`](./graph/drift-map.tsv) before it can receive a marker.
2. Add active drift to `Drift Registry` only when the modeled flow stops
   matching across plan and code.
3. Regenerate the graph with `scripts/generate-architecture-dot.sh` after
   any registry change.
4. A planned flow may enter both views only when the code-side view names its real substitute/missing carrier and an active drift marker; never invent a future path.

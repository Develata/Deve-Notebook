# WebLightPeer Architecture And Audit Unification

## TL;DR
> **Summary**: First update `deve-note plan/` to formally define `WebLightPeer`, then refactor code to match that design while closing all remaining audit findings in one coordinated pass.
> **Deliverables**:
> - Updated architecture/docs + acceptance cases for `WebLightPeer`
> - Browser peer identity/storage/auth/protocol redesign
> - Auth/CORS/cookie/proxy hardening
> - Multi-repo sync correctness, plugin capability closure, tree module split
> **Effort**: XL
> **Parallel**: YES - 4 waves
> **Critical Path**: T1 -> T2 -> T5 -> T6 -> T7 -> T8 -> F1-F4

## Context
### Original Request
User first asked to fix issues from `report/audit-2026-03-06.md`, then explicitly chose a more ambitious long-term direction: formalize a `WebLightPeer` architecture, update `deve-note plan/` first, and fold the remaining audit issues into the same total refactor.

### Interview Summary
- Direction chosen: do not reduce Web back to pure dashboard; instead create a first-class `WebLightPeer` design.
- Scope chosen: merge all remaining audit items into the same architecture plan.
- Test strategy chosen: `tests-after`.
- Planning consequence: docs become source-of-truth first, implementation follows docs.

### Metis Review (gaps addressed)
- Metis consultation was attempted twice but did not return within tool timeout; this plan therefore adds explicit guardrails for role boundaries, repo scoping, trust registration, and offline semantics to avoid hidden implementation judgment.
- Guardrail added: `WebLightPeer` is not a full peer; it must have explicit repo-scoped identity/vector semantics and explicit degraded/offline limits.
- Guardrail added: user session identity and peer identity remain separate concepts throughout docs and code.
- Guardrail added: all audit items must be absorbed into the same migration path, not fixed ad hoc.

## Work Objectives
### Core Objective
Define and implement a coherent `WebLightPeer` architecture that makes browser peer behavior explicit, secure, repo-scoped, and testable while simultaneously eliminating the outstanding security, protocol, sandbox, UI-state, and maintainability defects from the audit.

### Deliverables
- `deve-note plan/` updates for network, storage, auth, web UI, plugins, and acceptance cases
- Browser-side identity/storage/session architecture using `WebCrypto + IndexedDB`
- Repo-scoped sync handshake/snapshot/routing behavior with no `Uuid::nil()` placeholder paths
- Production-safe auth startup, cookie policy, CORS policy, and proxy/WS contract
- Closed plugin capability model and Rhai runtime limits
- Stable dashboard root behavior, exact cookie parsing, graceful browser storage fallback
- `crates/core/src/tree/manager.rs` split below fuse threshold without API regression

### Definition of Done (verifiable conditions with commands)
- Updated plan docs exist and are internally consistent: `deve-note plan/05_network.md`, `deve-note plan/04_storage.md`, `deve-note plan/09_auth.md`, `deve-note plan/08_ui_design_01_web.md`, `deve-note plan/11_plugins.md`, `deve-note plan/acceptance-cases/06_network.md`, `deve-note plan/acceptance-cases/08_auth.md`, `deve-note plan/acceptance-cases/10_plugins.md`
- Web implementation uses browser-safe durable storage design; no peer private identity remains in `localStorage`
- Sync paths use real repo identifiers end-to-end; no `Uuid::nil()` remains in targeted runtime paths
- Auth startup fails closed outside explicit dev mode; cookie/CORS/WS behavior matches docs
- Plugin host functions and Rhai runtime enforce declared capabilities and execution limits
- `cargo clippy --all-targets --all-features -- -D warnings` passes
- `cargo test` passes
- If `.opencode/command` changed during implementation, `sync-speckit.ps1` then `check-speckit-sync.ps1` pass

### Must Have
- Single coherent architecture for `WebLightPeer`
- Repo-scoped identity, vector, and sync routing
- Distinct user session auth vs peer auth model
- Agent-executable QA evidence for every task
- No file exceeds project fuse threshold after refactors

### Must NOT Have (guardrails, AI slop patterns, scope boundaries)
- Must NOT rationalize current `localStorage + SyncHello` behavior as final architecture
- Must NOT leave browser peer semantics implicit or undocumented
- Must NOT use `Uuid::nil()` as any runtime repo fallback in sync logic
- Must NOT preserve production auth fallback to `dev_default()`
- Must NOT add heavyweight browser/runtime dependencies that conflict with 768 MB deployment target
- Must NOT split work into multiple independent plans

## Verification Strategy
> ZERO HUMAN INTERVENTION - all verification is agent-executed.
- Test decision: `tests-after` with existing Rust test framework + targeted browser/manual-agent QA
- QA policy: Every task includes executable scenarios and evidence capture
- Evidence: `.sisyphus/evidence/task-{N}-{slug}.{ext}`

## Execution Strategy
### Parallel Execution Waves
> Target: 5-8 tasks per wave. Shared contracts land first.

Wave 1: architecture/docs contracts (`T1-T4`)
Wave 2: auth/session/browser identity foundation (`T5-T8`)
Wave 3: protocol/plugin/runtime/backend correctness (`T9-T12`)
Wave 4: structural cleanup + global regression (`T13-T14`)

### Dependency Matrix (full, all tasks)
`T1` blocks `T2-T14`
`T2` blocks `T5-T10`
`T3` blocks `T5-T8`
`T4` blocks `T11-T14`
`T5` blocks `T6-T8`
`T6` blocks `T7-T8`
`T7` blocks `T8-T12`
`T8` blocks `T14`
`T9` independent after `T1-T4`
`T10` depends on `T2-T3`
`T11` depends on `T4`
`T12` depends on `T2-T4`
`T13` depends on `T11`
`T14` depends on `T5-T13`

### Agent Dispatch Summary (wave -> task count -> categories)
- Wave 1 -> 4 tasks -> `writing`, `deep`
- Wave 2 -> 4 tasks -> `deep`, `unspecified-high`, `visual-engineering`
- Wave 3 -> 4 tasks -> `deep`, `ultrabrain`, `unspecified-high`
- Wave 4 -> 2 tasks -> `quick`, `deep`

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

- [x] 1. Define WebLightPeer terminology and invariants

  **What to do**: Update architecture vocabulary so `WebLightPeer`, `DashboardSession`, `PeerIdentity`, `RepoScopedVector`, `OfflineCache`, and `DegradedSyncMode` are defined once and reused consistently. Add explicit invariants for browser peer identity, repo scope, and online/offline limits.
  **Must NOT do**: Must NOT edit implementation code in this task; must NOT leave browser role described simultaneously as pure dashboard and peer.

  **Recommended Agent Profile**:
  - Category: `writing` - Reason: terminology + normative architecture text
  - Skills: `[]` - No special skill required
  - Omitted: `playwright` - Not needed for doc synthesis

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: `2-14` | Blocked By: none

  **References**:
  - Pattern: `deve-note plan/05_network.md:19` - Existing Web dashboard definition to replace
  - Pattern: `deve-note plan/09_auth.md:13` - Existing auth/session wording to align with role split
  - Pattern: `apps/web/src/hooks/use_core/mod.rs:34` - Current browser identity persistence behavior
  - Pattern: `apps/web/src/hooks/use_core/effects.rs:23` - Current browser sync handshake behavior

  **Acceptance Criteria**:
  - [ ] `deve-note plan/05_network.md` and dependent docs use one consistent role vocabulary for browser peer/session behavior
  - [ ] At least one invariants subsection explicitly states browser peer constraints and repo-scope rules

  **QA Scenarios**:
  ```text
  Scenario: Terminology is internally consistent
    Tool: Bash
    Steps: run a workspace content search for old conflicting phrases such as "stateless dashboard" and "not a P2P node" in updated target docs
    Expected: conflicting legacy wording is either removed or rephrased to fit WebLightPeer role
    Evidence: .sisyphus/evidence/task-1-terminology.txt

  Scenario: Invariants are explicit
    Tool: Bash
    Steps: search updated docs for "Invariant"/"不变量" and repo-scope terminology
    Expected: architecture docs contain explicit rules, not only descriptive prose
    Evidence: .sisyphus/evidence/task-1-terminology-error.txt
  ```

  **Commit**: NO | Message: `docs(architecture): define weblightpeer invariants` | Files: `deve-note plan/05_network.md`, `deve-note plan/09_auth.md`

- [ ] 2. Rewrite network protocol contract around repo-scoped WebLightPeer sync

  **What to do**: Update network design to formalize `WebLightPeer` handshake, `repo_id` semantics, reconnect flow, snapshot fallback, and proxy/relative-WS routing. Specify that browser peer state is repo-scoped and that `SyncHello`/`SyncRequest` must carry enough repo context for deterministic routing.
  **Must NOT do**: Must NOT preserve `3001..3005` scanning as primary production behavior; must NOT allow nil repo placeholders in protocol examples.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: protocol and state-model work
  - Skills: `[]` - No external library dependency for docs task
  - Omitted: `frontend-ui-ux` - UI design is not primary here

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: `5-10` | Blocked By: `1`

  **References**:
  - Pattern: `deve-note plan/05_network.md:32` - Existing role/port probing contract
  - Pattern: `deve-note plan/acceptance-cases/06_network.md:14` - Existing hardcoded proxy/port acceptance case
  - API/Type: `crates/core/src/sync/engine/handshake.rs:21` - Current nil repo placeholder in diff requests
  - API/Type: `crates/core/src/sync/engine/transfer/snapshot.rs:19` - Current repo_id ignored in snapshot path
  - API/Type: `apps/web/src/api/connection.rs` - Current WS URL probing behavior to replace

  **Acceptance Criteria**:
  - [ ] Updated network docs explicitly define `WebLightPeer` handshake and repo-scoped routing
  - [ ] Acceptance cases no longer rely on hardcoded `3001..3005` as normative production behavior

  **QA Scenarios**:
  ```text
  Scenario: Protocol examples reject nil repo placeholders
    Tool: Bash
    Steps: search updated network docs and acceptance cases for "Uuid::nil" and stale hardcoded port-probe language
    Expected: no normative protocol text keeps placeholder repo routing or legacy scanning as default behavior
    Evidence: .sisyphus/evidence/task-2-network.txt

  Scenario: Repo-scoped routing is documented end-to-end
    Tool: Bash
    Steps: search for `repo_id`, `SyncHello`, `Snapshot`, and `relative /ws` in updated docs
    Expected: all appear in the same architecture/acceptance chain
    Evidence: .sisyphus/evidence/task-2-network-error.txt
  ```

  **Commit**: NO | Message: `docs(network): formalize repo-scoped weblightpeer sync` | Files: `deve-note plan/05_network.md`, `deve-note plan/acceptance-cases/06_network.md`

- [ ] 3. Define browser storage and trust model for WebLightPeer

  **What to do**: Update storage/auth/web UI docs to move browser peer durability from `localStorage` to `WebCrypto + IndexedDB`, separate UI prefs from peer identity and offline cache, and define trust registration/recovery behavior. Include data classification and lifecycle rules.
  **Must NOT do**: Must NOT allow peer private material to remain specified in `localStorage`; must NOT blur login cookie with peer identity.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: cross-cutting storage/auth model
  - Skills: `[]` - Repo context is sufficient
  - Omitted: `git-master` - No git operation required

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: `5-8,10` | Blocked By: `1`

  **References**:
  - Pattern: `apps/web/src/hooks/use_core/mod.rs:43` - Current `localStorage` access and panic point
  - Pattern: `deve-note plan/05_network.md:22` - Existing browser storage statement
  - Pattern: `deve-note plan/09_auth.md:14` - Existing session/JWT wording
  - Pattern: `deve-note plan/08_ui_design_01_web.md` - UI state document to extend with sync state presentation

  **Acceptance Criteria**:
  - [ ] Docs distinguish `UI prefs`, `user session`, `peer identity`, and `offline cache`
  - [ ] Docs specify browser durable storage primitives and recovery semantics

  **QA Scenarios**:
  ```text
  Scenario: Storage classes are explicit
    Tool: Bash
    Steps: search updated docs for `localStorage`, `IndexedDB`, `WebCrypto`, `UI prefs`, and `peer identity`
    Expected: each storage class has explicit allowed/forbidden usage
    Evidence: .sisyphus/evidence/task-3-storage.txt

  Scenario: Auth and storage are not conflated
    Tool: Bash
    Steps: inspect updated auth/storage docs for distinct definitions of session token vs peer identity
    Expected: user login and browser peer registration are separate documented flows
    Evidence: .sisyphus/evidence/task-3-storage-error.txt
  ```

  **Commit**: NO | Message: `docs(storage): define browser peer durability model` | Files: `deve-note plan/04_storage.md`, `deve-note plan/09_auth.md`, `deve-note plan/08_ui_design_01_web.md`

- [ ] 4. Rewrite acceptance suites to match the new architecture and audit closure

  **What to do**: Update acceptance cases for network, auth, and plugins so they validate `WebLightPeer`, fail-closed auth, strict cookie parsing, repo-scoped sync, capability gates, and runtime limits. Remove stale legacy assumptions from acceptance inputs.
  **Must NOT do**: Must NOT leave acceptance cases that validate generated secret fallback or legacy browser-as-dashboard assumptions.

  **Recommended Agent Profile**:
  - Category: `writing` - Reason: executable acceptance text and scenario rewriting
  - Skills: `[]` - No external docs needed
  - Omitted: `playwright` - Runtime verification belongs to later tasks

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: `11-14` | Blocked By: `1`

  **References**:
  - Test: `deve-note plan/acceptance-cases/06_network.md` - Network acceptance base
  - Test: `deve-note plan/acceptance-cases/08_auth.md:4` - Existing auth startup case to replace
  - Test: `deve-note plan/acceptance-cases/10_plugins.md:16` - Existing default-deny plugin case to expand
  - Pattern: `report/audit-2026-03-06.md` - Source list of required audit closures

  **Acceptance Criteria**:
  - [ ] Acceptance cases cover every audit issue that remains in implementation scope
  - [ ] Acceptance cases align with updated docs and no longer encode stale ports, secret fallback, or placeholder repo behavior

  **QA Scenarios**:
  ```text
  Scenario: Audit-to-acceptance coverage exists
    Tool: Bash
    Steps: compare audit issue IDs against updated acceptance case files
    Expected: each audit item has at least one matching acceptance scenario
    Evidence: .sisyphus/evidence/task-4-acceptance.txt

  Scenario: Legacy expectations are removed
    Tool: Bash
    Steps: search acceptance files for `generated random secret`, `3001..3005`, and other stale phrases
    Expected: no obsolete acceptance wording remains in target files
    Evidence: .sisyphus/evidence/task-4-acceptance-error.txt
  ```

  **Commit**: NO | Message: `docs(acceptance): align suites with weblightpeer migration` | Files: `deve-note plan/acceptance-cases/06_network.md`, `deve-note plan/acceptance-cases/08_auth.md`, `deve-note plan/acceptance-cases/10_plugins.md`

- [ ] 5. Make auth startup and deployment contract fail-closed

  **What to do**: Refactor auth/config bootstrap so production startup fails when required auth env is missing, while explicit dev mode still supports dev auth. Make cookie security policy, logout cookie attributes, and CORS origin handling conform to documented environment-driven rules.
  **Must NOT do**: Must NOT keep silent fallback to `dev_default()` outside explicit dev mode; must NOT keep production behavior tied to localhost-only assumptions.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: security + deployment contract
  - Skills: `[]` - Repo context sufficient
  - Omitted: `frontend-ui-ux` - Primarily backend/security work

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: `6-8,14` | Blocked By: `2,3`

  **References**:
  - Pattern: `apps/cli/src/server/router.rs:70` - Current auth fallback entrypoint
  - API/Type: `crates/core/src/security/auth/config.rs` - `from_env()` + `dev_default()` behavior
  - Pattern: `apps/cli/src/server/auth/handlers.rs:143` - Hardcoded `secure(false)` cookie
  - Pattern: `apps/cli/src/server/setup.rs:15` - Hardcoded localhost CORS builder
  - Pattern: `deve-note plan/09_auth.md:34` - Target production/development policy text

  **Acceptance Criteria**:
  - [ ] Startup fails outside explicit dev mode when required auth secrets are absent
  - [ ] Cookie `Secure`/`SameSite` and CORS policy become environment-driven and match docs

  **QA Scenarios**:
  ```text
  Scenario: Production startup fails closed
    Tool: Bash
    Steps: run server startup with production env and missing `AUTH_SECRET`/`AUTH_PASS`
    Expected: startup exits non-zero and logs explicit auth configuration failure
    Evidence: .sisyphus/evidence/task-5-auth.txt

  Scenario: Dev mode remains explicit, not implicit
    Tool: Bash
    Steps: run server startup with explicit dev mode toggle and missing auth env
    Expected: server starts only when dev mode is explicitly enabled; otherwise it refuses
    Evidence: .sisyphus/evidence/task-5-auth-error.txt
  ```

  **Commit**: YES | Message: `fix(auth): fail closed outside explicit dev mode` | Files: `apps/cli/src/server/router.rs`, `crates/core/src/security/auth/config.rs`, `apps/cli/src/server/auth/handlers.rs`, `apps/cli/src/server/setup.rs`

- [ ] 6. Build WebLightPeer identity and durable browser storage substrate

  **What to do**: Replace browser peer identity persistence from `localStorage` with a `WebCrypto + IndexedDB` backed substrate. Separate peer identity material, repo-scoped vector/cache metadata, and UI preferences. Introduce graceful capability fallback when durable browser storage is unavailable.
  **Must NOT do**: Must NOT persist peer private key material in `localStorage`; must NOT panic when `localStorage`/browser storage APIs are unavailable.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: browser crypto/storage architecture
  - Skills: [`frontend-ui-ux`] - Helpful for surfacing degraded browser state cleanly
  - Omitted: `playwright` - Not needed during implementation task itself

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: `7-8,10,14` | Blocked By: `3,5`

  **References**:
  - Pattern: `apps/web/src/hooks/use_core/mod.rs:43` - Current browser identity persistence and panic path
  - Pattern: `deve-note plan/04_storage.md` - Target storage policy after T3
  - Pattern: `deve-note plan/08_ui_design_01_web.md` - Degraded/offline UI obligations
  - External: `WebCrypto` and IndexedDB browser primitives as defined by updated docs in T3

  **Acceptance Criteria**:
  - [ ] Browser peer identity uses the new durable substrate defined in docs
  - [ ] Storage-unavailable environments degrade gracefully without panic and expose a documented limited mode

  **QA Scenarios**:
  ```text
  Scenario: Browser identity survives reload through durable substrate
    Tool: Playwright
    Steps: open web app, initialize peer identity, reload page, inspect displayed/derived peer identity state
    Expected: identity persists via documented durable mechanism without using `localStorage` for private peer material
    Evidence: .sisyphus/evidence/task-6-storage.png

  Scenario: Storage restricted mode degrades safely
    Tool: Playwright
    Steps: run browser context with storage restrictions/private mode, load app, trigger initialization
    Expected: no panic; UI enters documented degraded mode with limited sync capability
    Evidence: .sisyphus/evidence/task-6-storage-error.png
  ```

  **Commit**: YES | Message: `feat(web): add durable weblightpeer identity substrate` | Files: `apps/web/src/hooks/use_core/mod.rs`, new browser storage modules under `apps/web/src/`

- [ ] 7. Enforce repo-scoped sync routing across handshake, listing, merge, and snapshot

  **What to do**: Remove all runtime `Uuid::nil()` placeholders, propagate actual `repo_id` through browser/server handshake and sync flows, and make snapshot/listing/merge paths honor repo routing instead of ignoring request scope. Update related types/contracts as needed.
  **Must NOT do**: Must NOT leave any nil repo fallback in hot sync paths; must NOT accept repo-less `SyncHello` semantics once WebLightPeer is formalized.

  **Recommended Agent Profile**:
  - Category: `ultrabrain` - Reason: protocol correctness across multiple modules
  - Skills: `[]` - Pure repo reasoning
  - Omitted: `frontend-ui-ux` - Protocol work is primary

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: `8,12,14` | Blocked By: `2,6`

  **References**:
  - API/Type: `crates/core/src/sync/engine/handshake.rs:21` - Nil repo placeholder in diff path
  - API/Type: `apps/cli/src/server/handlers/listing.rs:41` - Nil repo placeholder for remote listing
  - API/Type: `apps/cli/src/server/handlers/merge.rs:119` - Nil repo fallback from repo info
  - API/Type: `crates/core/src/sync/engine/transfer/snapshot.rs:19` - Request repo ignored in snapshot source selection
  - Pattern: `deve-note plan/05_network.md` - Repo-scoped protocol contract from T2

  **Acceptance Criteria**:
  - [ ] Targeted sync/runtime paths use real repo identifiers end-to-end
  - [ ] Repo mismatch and unknown repo cases fail explicitly instead of silently defaulting

  **QA Scenarios**:
  ```text
  Scenario: Multi-repo sync routes by repo_id
    Tool: Bash
    Steps: create or select two repos, trigger sync/list/snapshot requests for each, inspect outputs/logs
    Expected: each request returns data from the addressed repo only; no cross-repo leakage
    Evidence: .sisyphus/evidence/task-7-repo.txt

  Scenario: Missing/unknown repo is rejected
    Tool: Bash
    Steps: send sync/list/snapshot request with absent or invalid repo_id
    Expected: explicit error or rejection path; no fallback to nil/default repo
    Evidence: .sisyphus/evidence/task-7-repo-error.txt
  ```

  **Commit**: YES | Message: `fix(sync): enforce repo-scoped routing` | Files: `crates/core/src/sync/engine/handshake.rs`, `crates/core/src/sync/engine/transfer/snapshot.rs`, `apps/cli/src/server/handlers/listing.rs`, `apps/cli/src/server/handlers/merge.rs`

- [ ] 8. Refactor web connection, login, and root-state behavior around DashboardSession + WebLightPeer

  **What to do**: Introduce clear browser session behavior: relative `/ws` first, login/session flow consistent with auth contract, `DashboardSession` separate from peer identity, and stable Home/Dashboard state that is not overridden by document-list refresh. Update any necessary UI status surfaces for reconnect/degraded mode.
  **Must NOT do**: Must NOT let `DocList` auto-select the first doc after explicit Home navigation; must NOT keep legacy port scanning as the primary runtime path.

  **Recommended Agent Profile**:
  - Category: `visual-engineering` - Reason: UI state + browser connection flow
  - Skills: [`frontend-ui-ux`] - Needed for deliberate state/status UX
  - Omitted: `git-master` - Not relevant during implementation

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: `14` | Blocked By: `5,6,7`

  **References**:
  - Pattern: `apps/web/src/api/connection.rs` - Current absolute WS/port probing logic
  - Pattern: `apps/web/src/hooks/use_core/effects.rs:50` - Current browser `SyncHello` trigger
  - Pattern: `apps/web/src/hooks/use_core/effects_msg.rs:24` - Current `DocList` auto-select behavior
  - Pattern: `apps/web/src/api/output.rs:71` - Current queued `ListDocs` injection path
  - Pattern: `apps/web/src/components/main_layout.rs:108` - Home button state reset

  **Acceptance Criteria**:
  - [ ] Browser connection uses documented relative/session-first behavior
  - [ ] Home/Dashboard remains stable after explicit navigation even when docs refresh or reconnect occurs

  **QA Scenarios**:
  ```text
  Scenario: Home view stays on Home across refresh messages
    Tool: Playwright
    Steps: open app, navigate Home, trigger reconnect or doc list refresh, observe selected state
    Expected: Home remains selected; no forced jump to first document
    Evidence: .sisyphus/evidence/task-8-dashboard.png

  Scenario: Browser falls back cleanly when session or repo context is invalid
    Tool: Playwright
    Steps: invalidate session or repo selection, reload or reconnect
    Expected: user is routed to the documented auth/session recovery path; no hidden port-scan or stale-doc selection behavior
    Evidence: .sisyphus/evidence/task-8-dashboard-error.png
  ```

  **Commit**: YES | Message: `feat(web): align session flow with weblightpeer model` | Files: `apps/web/src/api/connection.rs`, `apps/web/src/hooks/use_core/effects.rs`, `apps/web/src/hooks/use_core/effects_msg.rs`, `apps/web/src/api/output.rs`, `apps/web/src/components/main_layout.rs`

- [ ] 9. Close plugin capability gaps and add Rhai runtime quotas

  **What to do**: Extend plugin capability schema to cover `search`, `skill`, `mcp`, and `project_tree`; thread those checks through host registration and host functions; add Rhai execution limits such as max operations and timeout/interruption support consistent with the documented quota model.
  **Must NOT do**: Must NOT leave default-deny claims unsupported by runtime code; must NOT add capability bypasses through helper functions.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: security model + runtime enforcement
  - Skills: `[]` - Internal design only
  - Omitted: `frontend-ui-ux` - Backend runtime work

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: `14` | Blocked By: `1,4`

  **References**:
  - API/Type: `crates/core/src/plugin/manifest.rs` - Capability schema to extend
  - Pattern: `crates/core/src/plugin/runtime/host/mod.rs:80` - Unconditional host registrations
  - Pattern: `crates/core/src/plugin/runtime/host/fs.rs:55` - `get_project_tree()` ungated
  - Pattern: `crates/core/src/plugin/runtime/host/search.rs:21` - Search host API without capabilities
  - Pattern: `crates/core/src/plugin/runtime/host/skill.rs:42` - Skill host API without capabilities
  - Pattern: `crates/core/src/plugin/runtime/rhai_v1.rs:40` - Missing runtime operation/time limits

  **Acceptance Criteria**:
  - [ ] Plugin manifest/capability model covers every exposed host-function family
  - [ ] Rhai scripts are bounded by explicit execution limits and denied capabilities fail clearly

  **QA Scenarios**:
  ```text
  Scenario: Missing capability denies host function access
    Tool: Bash
    Steps: run representative plugin calls for search/skill/mcp/project_tree without declared permissions
    Expected: each call fails with explicit capability-denied output
    Evidence: .sisyphus/evidence/task-9-plugin.txt

  Scenario: Runaway script is interrupted
    Tool: Bash
    Steps: execute a Rhai script designed to exceed operation/time limits
    Expected: runtime aborts deterministically and reports quota/timeout failure
    Evidence: .sisyphus/evidence/task-9-plugin-error.txt
  ```

  **Commit**: YES | Message: `fix(plugin): enforce capability gates and runtime limits` | Files: `crates/core/src/plugin/manifest.rs`, `crates/core/src/plugin/runtime/host/*.rs`, `crates/core/src/plugin/runtime/rhai_v1.rs`

- [ ] 10. Harden cookie parsing and browser storage fallback edge cases

  **What to do**: Replace loose cookie extraction with exact-name matching in both HTTP and WS auth paths; ensure browser storage fallback paths behave gracefully and match the new WebLightPeer degraded-mode rules.
  **Must NOT do**: Must NOT duplicate subtly different cookie parsers across HTTP and WS paths after the fix; must NOT rely on `expect()` for browser storage APIs.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: small but security-sensitive cross-layer hardening
  - Skills: `[]` - Internal repo work only
  - Omitted: `playwright` - Browser verification happens in QA section

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: `14` | Blocked By: `3,5,6`

  **References**:
  - Pattern: `apps/cli/src/server/auth/middleware.rs:77` - Loose cookie extraction
  - Pattern: `apps/cli/src/server/ws/mod.rs:121` - Duplicate loose WS cookie extraction
  - Pattern: `apps/web/src/hooks/use_core/mod.rs:43` - Existing storage `expect()` path
  - Pattern: `deve-note plan/acceptance-cases/08_auth.md` - Auth edge-case acceptance text

  **Acceptance Criteria**:
  - [ ] HTTP and WS auth use exact cookie-name matching with shared or equivalent deterministic logic
  - [ ] Browser storage fallback paths no longer panic and match documented degraded behavior

  **QA Scenarios**:
  ```text
  Scenario: Similar cookie names do not authenticate
    Tool: Bash
    Steps: send requests carrying `token_csrf` or `token_backup` but not exact `token`
    Expected: auth fails; only exact cookie name is accepted
    Evidence: .sisyphus/evidence/task-10-cookie.txt

  Scenario: Browser storage failure no longer panics
    Tool: Playwright
    Steps: simulate restricted storage environment and initialize app auth/storage flow
    Expected: app remains responsive and enters documented fallback mode
    Evidence: .sisyphus/evidence/task-10-cookie-error.png
  ```

  **Commit**: YES | Message: `fix(auth): harden cookie parsing and storage fallback` | Files: `apps/cli/src/server/auth/middleware.rs`, `apps/cli/src/server/ws/mod.rs`, `apps/web/src/hooks/use_core/mod.rs`

- [ ] 11. Update plugin and auth docs to match the implemented security boundary

  **What to do**: After runtime/auth changes land, finalize `deve-note plan/11_plugins.md` and remaining auth acceptance/doc text so the written capability model, session model, and quota policy exactly match implementation.
  **Must NOT do**: Must NOT leave docs describing capabilities or session behavior that code does not enforce.

  **Recommended Agent Profile**:
  - Category: `writing` - Reason: post-implementation contract alignment
  - Skills: `[]` - Repo context sufficient
  - Omitted: `git-master` - No git operation required

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: `13-14` | Blocked By: `5,9,10`

  **References**:
  - Pattern: `deve-note plan/11_plugins.md:32` - Existing capability model text
  - Pattern: `deve-note plan/09_auth.md` - Auth/session contract to align
  - Test: `deve-note plan/acceptance-cases/10_plugins.md` - Plugin acceptance suite
  - Test: `deve-note plan/acceptance-cases/08_auth.md` - Auth acceptance suite

  **Acceptance Criteria**:
  - [ ] Plugin and auth docs match shipped capability and security behavior exactly
  - [ ] No stale text remains describing old capability fields, old cookie policy, or old startup fallback behavior

  **QA Scenarios**:
  ```text
  Scenario: Doc-to-code security alignment holds
    Tool: Bash
    Steps: compare updated docs against changed capability fields and auth configuration names
    Expected: doc terms and config names match code exactly
    Evidence: .sisyphus/evidence/task-11-docsync.txt

  Scenario: Stale security wording is gone
    Tool: Bash
    Steps: search target docs for removed behavior such as implicit dev fallback or incomplete capability lists
    Expected: no stale security wording remains
    Evidence: .sisyphus/evidence/task-11-docsync-error.txt
  ```

  **Commit**: NO | Message: `docs(security): align auth and plugin contracts` | Files: `deve-note plan/11_plugins.md`, `deve-note plan/09_auth.md`, `deve-note plan/acceptance-cases/08_auth.md`, `deve-note plan/acceptance-cases/10_plugins.md`

- [ ] 12. Realign command/spec synchronization and stale protocol references

  **What to do**: Update any touched command/spec source-of-truth files required by the refactor, then run mandatory speckit sync/check commands if `.opencode/command` changes. Sweep stale protocol references across docs/code comments introduced by old port/proxy/browser assumptions.
  **Must NOT do**: Must NOT modify command-derived files without performing required sync/check sequence; must NOT leave stale protocol comments that contradict the new plan.

  **Recommended Agent Profile**:
  - Category: `unspecified-high` - Reason: cross-cutting consistency cleanup
  - Skills: `[]` - Repo context only
  - Omitted: `playwright` - Not relevant

  **Parallelization**: Can Parallel: YES | Wave 3 | Blocks: `14` | Blocked By: `2,4,7,8,11`

  **References**:
  - Pattern: `.opencode/command` - SOT if touched by implementation
  - Pattern: `deve-note plan/05_network.md`, `deve-note plan/09_auth.md` - Docs likely to introduce changed command/spec references
  - Pattern: `report/audit-2026-03-06.md` - M3 stale-doc issue source

  **Acceptance Criteria**:
  - [ ] All touched command/spec artifacts are synchronized
  - [ ] No stale protocol/auth/proxy references remain in targeted docs/comments

  **QA Scenarios**:
  ```text
  Scenario: Speckit sync passes when required
    Tool: Bash
    Steps: if `.opencode/command` changed, run `sync-speckit.ps1` then `check-speckit-sync.ps1`
    Expected: both commands succeed; if `.opencode/command` unchanged, record skip with reason
    Evidence: .sisyphus/evidence/task-12-sync.txt

  Scenario: Stale references are removed
    Tool: Bash
    Steps: search for retired port/proxy/auth wording across touched files
    Expected: no known stale references remain in the targeted scope
    Evidence: .sisyphus/evidence/task-12-sync-error.txt
  ```

  **Commit**: NO | Message: `docs(protocol): sync command and reference contracts` | Files: touched spec/command files only

- [ ] 13. Split tree manager into submodules below fuse threshold

  **What to do**: Refactor `crates/core/src/tree/manager.rs` into smaller submodules while preserving the public `TreeManager` API and behavior. Extract initialization/build/helper logic so no file exceeds the project fuse threshold.
  **Must NOT do**: Must NOT change exported `TreeManager` semantics as a side effect; must NOT leave any resulting source file above 250 lines.

  **Recommended Agent Profile**:
  - Category: `quick` - Reason: constrained refactor with clear structure map
  - Skills: `[]` - Repo context sufficient
  - Omitted: `frontend-ui-ux` - Not relevant

  **Parallelization**: Can Parallel: YES | Wave 4 | Blocks: `14` | Blocked By: `4,11`

  **References**:
  - Pattern: `crates/core/src/tree/manager.rs` - Oversized source file to split
  - Pattern: `crates/core/src/tree/mod.rs` - Module declarations/re-exports to preserve
  - Pattern: `crates/core/src/tree/ops.rs` - Existing helper-module pattern to follow

  **Acceptance Criteria**:
  - [ ] `TreeManager` public API remains intact
  - [ ] No resulting tree module file exceeds the 250-line fuse threshold

  **QA Scenarios**:
  ```text
  Scenario: Tree API still compiles and tests
    Tool: Bash
    Steps: run targeted tree-related tests or package tests after refactor
    Expected: behavior remains unchanged and compilation succeeds
    Evidence: .sisyphus/evidence/task-13-tree.txt

  Scenario: File-size fuse is respected
    Tool: Bash
    Steps: inspect resulting tree module file line counts
    Expected: all touched tree source files are <= 250 lines
    Evidence: .sisyphus/evidence/task-13-tree-error.txt
  ```

  **Commit**: YES | Message: `refactor(tree): split manager into submodules` | Files: `crates/core/src/tree/*`

- [ ] 14. Run integrated regression, browser QA, and release-readiness verification

  **What to do**: Execute full post-change verification covering Rust checks, targeted browser QA, acceptance-case spot validation, and end-to-end audit closure review. Confirm docs, implementation, and runtime behavior match.
  **Must NOT do**: Must NOT declare completion with partial green status; must NOT skip browser/session verification because backend tests pass.

  **Recommended Agent Profile**:
  - Category: `deep` - Reason: cross-cutting release gate
  - Skills: [`playwright`] - Required for browser/session verification
  - Omitted: `git-master` - Verification only

  **Parallelization**: Can Parallel: NO | Wave 4 | Blocks: `F1-F4` | Blocked By: `5-13`

  **References**:
  - Test: `deve-note plan/acceptance-cases/06_network.md`, `deve-note plan/acceptance-cases/08_auth.md`, `deve-note plan/acceptance-cases/10_plugins.md` - Acceptance targets
  - Pattern: `report/audit-2026-03-06.md` - Final audit closure checklist
  - Pattern: `AGENTS.md` - Build/test/clippy constraints and file-size rules

  **Acceptance Criteria**:
  - [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
  - [ ] `cargo test` passes
  - [ ] Browser QA validates WebLightPeer/session/root-state behaviors and auth edge cases
  - [ ] Final audit checklist shows all targeted issues closed or intentionally superseded by the new architecture

  **QA Scenarios**:
  ```text
  Scenario: Full Rust verification passes
    Tool: Bash
    Steps: run `cargo clippy --all-targets --all-features -- -D warnings` then `cargo test`
    Expected: both commands pass cleanly
    Evidence: .sisyphus/evidence/task-14-rust.txt

  Scenario: Browser and acceptance regression passes
    Tool: Playwright
    Steps: execute representative flows for login, reconnect, repo switching, degraded mode, and dashboard root stability using updated acceptance cases
    Expected: observed behavior matches updated docs and no audited regression reproduces
    Evidence: .sisyphus/evidence/task-14-browser.png
  ```

  **Commit**: NO | Message: `test(regression): verify weblightpeer migration and audit closure` | Files: none

## Final Verification Wave (4 parallel agents, ALL must APPROVE)
- [ ] F1. Plan Compliance Audit - oracle
- [ ] F2. Code Quality Review - unspecified-high
- [ ] F3. Real Manual QA - unspecified-high (+ playwright if UI)
- [ ] F4. Scope Fidelity Check - deep

## Commit Strategy
- Commit after each completed execution wave when the workspace is green.
- Suggested sequence:
  - `docs(architecture): formalize WebLightPeer contracts`
  - `feat(web): add WebLightPeer identity and session model`
  - `fix(sync): enforce repo-scoped routing and auth hardening`
  - `fix(plugin): gate host capabilities and runtime limits`
  - `refactor(tree): split tree manager and close audit regressions`

## Success Criteria
- Web browser role is explicit, documented, and implemented as `WebLightPeer`
- Remaining audit findings are eliminated as part of the migration, not patched around
- Browser/session/protocol/security behaviors match updated acceptance cases
- Repository remains buildable, testable, and within file-size constraints

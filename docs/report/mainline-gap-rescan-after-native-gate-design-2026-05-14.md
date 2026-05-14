# Mainline Gap Rescan After Native Gate Design - 2026-05-14

本报告记录 native packaging gate design 后的主线缺口复扫。`docs/plan/` 仍是唯一权威；本批次不修改 plan，不打开 native packaging gate。

## Scope

- Source of truth: `docs/plan/`.
- Cross-check inputs: `docs/features/operations/`, `docs/acceptance-cases/`, current code, guard scripts, and three read-only explorer audits.
- Non-goal: edit `docs/plan/`, introduce Tauri, re-enable MCP, or start broad UI/runtime rewrites.

## Baseline

Ran:

- `bash scripts/plan-coverage.sh --summary-missing-plan-ref`
- `bash scripts/check-acceptance-bindings.sh`
- `bash scripts/check-feature-operation-paths.sh`
- `bash scripts/check-ui-focus-baseline.sh`
- `bash scripts/check-ui-token-baseline.sh`
- `bash scripts/check-ui-z-index-baseline.sh`
- `bash scripts/check-release-baseline.sh`
- `bash scripts/check-native-track-boundary.sh`
- `bash scripts/check-native-packaging-gate.sh`

Results:

- Plan coverage: `0` blocking violations, `17` known soft size warnings.
- Acceptance binding scan: `0` unbound cases reported by the current script.
- Feature operation path scan: pass.
- UI focus/token/z-index guards: pass.
- Release/native/no-packaging guards: pass.

## Accepted Current Gaps

### G1. Auth Token Subject Uses Hardcoded `admin`

- Priority: P1.
- Plan basis: `09_auth.md#auth-http-endpoints`, `09_auth.md#jwt-cookie-contract`.
- Finding: login checks `AUTH_USER`, but token issuing still writes `sub = "admin"`.
- Impact: non-default `AUTH_USER` deployments can authenticate but `/api/auth/me` reports the wrong current user identity.
- Next action: pass configured username into token issuing and add a non-default `AUTH_USER` regression.

### G2. Invalid `AUTH_PASS` PHC Does Not Fail Closed At Startup

- Priority: P1.
- Plan basis: `09_auth.md#auth-config`, `09_auth.md#password-hashing`.
- Finding: production config validates presence of `AUTH_PASS` but does not parse the Argon2 PHC before accepting startup.
- Impact: malformed production password hash fails later during login instead of failing closed during auth material load.
- Next action: validate `AUTH_PASS` with `PasswordHash::new` in `AuthConfig::from_env`.

### G3. WS Versioned JSON Text Frame Is Not Fully Debug-Gated

- Priority: P1.
- Plan basis: `05_network.md#protocol-versioning`.
- Finding: legacy JSON text is debug-gated, but versioned JSON text still routes in the default runtime path.
- Impact: production can accept a text-frame path that plan limits to explicit debug compatibility.
- Next action: gate all client text WS frames behind an explicit debug flag; production default remains versioned binary.

### G4. Mobile Footer PendingAck Count Is Not Scope-Filtered

- Priority: P2.
- Plan basis: `16_web_thin_client_ledger.md#pending-overlay-lifecycle`.
- Finding: desktop bottom bar filters pending edits by current repo/scope; mobile footer counts by `doc_id` only.
- Impact: stale-scope pending edits can display `PendingAck` in mobile status after repo/scope switch.
- Next action: reuse `pending_count_for_doc_in_scope` in mobile footer and add stale-scope regression.

### G5. Acceptance ID Parser Truncates Letter-Suffixed IDs

- Priority: P2.
- Plan basis: `docs/plan/AGENTS.md` Layer 3.
- Finding: `scripts/check-acceptance-bindings.sh` only matches IDs ending in digits, so `CMD-007A` and `CMD-007B` are parsed as `CMD-007`.
- Impact: binding counts can falsely look complete while two concrete cases are invisible to the scanner.
- Next action: extend the case-id regex and add a guard against ID truncation.

### G6. CLI Baseline Script Misses Some Baseline CLI Surfaces

- Priority: P2.
- Plan basis: `12_commands.md#cli-commands`.
- Finding: implemented commands such as `graph`, `verify-p2p`, `seed`, and `node-check` are not fully covered by `check-cli-settings-baseline.sh`.
- Impact: CLI contract can regress without the baseline script noticing.
- Next action: expand the script to cover command variants and help/dispatch surfaces already required by acceptance cases.

### G7. `REL-001` Still References A Non-Current `dist/` Release Artifact

- Priority: P2.
- Plan basis: `15_release.md#github-release-workflow`.
- Finding: acceptance case `REL-001` runs `ls dist` and expects `v1.0.0`, while current release plan centers on tag-triggered `release.yml` and Docker image publishing; native binaries remain deferred.
- Impact: release acceptance wording is stale relative to the current release contract.
- Next action: update `REL-001` to check release workflow trigger and Docker metadata, or explicitly mark native binary artifact as deferred.

## Larger Design Followups

These are real but should not be mixed with the small hardening batch:

- Source Control HTTP mutation/read scope gate: requires a scoped HTTP token or consolidation onto WS scoped path.
- Modal focus unification: requires shared modal shell migration across settings, pending navigation, merge, and future dialogs.
- Peer identity retry UI: requires structured peer identity state and retry action through bootstrap plus handshake plus writer gate.
- Watcher lifecycle: requires a decision whether watcher is active-scope service or all-local-repo service, then stop/drain tests.

## Rejected As Current Gaps

- Native packaging and Tauri dependencies remain deferred until the native packaging gate is explicitly opened.
- Live Preview, Milkdown, arbitrary HTML, wikilink/footnote/emoji/highlight semantics, and complete virtual rendering remain outside current baseline unless the plan is reopened.
- Acceptance binding hard-block semantics are a tooling policy issue; current repo intentionally supports automated, feature-walkthrough, and manual binding classes. Tightening this requires a separate plan-authorized decision.

## Decision

Next implementation batch should start with G1 and G2 together. They are small, high-signal auth hardening fixes with direct plan authority and limited blast radius.

After G1/G2: handle G3 WS text-frame debug gate, then G4 mobile PendingAck scope filtering, then the acceptance/release script cleanup items.

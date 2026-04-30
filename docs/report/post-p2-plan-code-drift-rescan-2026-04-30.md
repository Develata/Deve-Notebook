# Post-P2 Plan/Code Drift Rescan - 2026-04-30

## Scope

Rescanned current plan/code alignment after Native AI Chat and Browser UI prefs consolidation.

## Closed In This Batch

- Source Control baseline script no longer checks retired `Git: Push` wording as if all Git palette commands were absent; current `Git: Push Mirror` CLI-only notice remains valid.
- Browser `scope_prefs` now stores only the last `repo_name` display alias. It no longer persists `repo_id`, remote branch / peer id, `scope_nonce`, or authority identifiers in UI prefs.
- `deve init` config template now matches current `config.example.toml` schema for `merge_strategy`, `[ui]`, `[ai]`, and `[ai.agent_bridge]`.
- Trusted CLI policy now requires `AGENT_CLI_PATH` to point to an existing executable file, and non-zero CLI exit is treated as request failure.
- Dev runbook and scripts registry now include `scripts/check-browser-prefs-boundary.sh`.

## Remaining Priority Gaps

- P0: `ai.mode` / `ai.native_enabled` need an effective runtime decision. Config parses them, but Web currently initializes Native mode directly and `ai.native_enabled=false` does not disable the Native path.
- P0: Desktop/Mobile plan chapters still contain post-gate Tauri/embedded-service/offline-first MUST language after current-boundary sections that explicitly say no Tauri/process runtime is active.
- P1: Graph remains a read-only repo-local summary panel. Before opening renderer dependencies, acceptance should cover blocked/local-only/degraded/empty/error states.
- P1: `AgentBridgePolicy` still documents env override aliases through code rather than the settings plan table; if those env vars remain supported, they should be named explicitly in plan.
- P2: UI/E2E acceptance for AI/settings remains mostly static-script based.

## Verification

- `scripts/check-source-control-baseline.sh`
- `scripts/check-ai-baseline.sh`
- `scripts/check-cli-settings-baseline.sh`
- `scripts/check-browser-prefs-boundary.sh`
- `scripts/check-dev-runbook-baseline.sh`
- `scripts/check-native-track-boundary.sh`
- `scripts/check-graph-baseline.sh`
- `scripts/check-release-baseline.sh`
- `cargo fmt --check`


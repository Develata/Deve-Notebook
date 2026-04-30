<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 | Updated: 2026-04-24 -->

# scripts

## Purpose

Build and lint utility scripts for Deve-Notebook. Provides low-memory lint configurations for resource-constrained environments.

## Key Files

| File | Description |
|------|-------------|
| `plan-coverage.sh` | Plan-code coverage, file-size fuse, i18n leak, and acceptance binding checks |
| `lint-low-mem.cmd` | Windows CMD script — runs clippy with reduced memory |
| `lint-low-mem.ps1` | PowerShell script — runs clippy with reduced memory |
| `check-architecture-registry.sh` | Verifies operation registry, acceptance refs, drift map, Lisp IDs, and graph spines stay aligned |
| `check-ws-structured-errors.sh` | Verifies WS protocol errors remain structured as `ServerError`/`ServerErrorCode` |
| `check-auth-unauthorized-state.sh` | Verifies auth failures map to Unauthorized instead of reconnect/disconnected UI |
| `check-auth-baseline.sh` | Verifies Auth startup, cookie/JWT/status, WS 401, rate-limit, headers, and frontend session-expired contracts |
| `check-network-baseline.sh` | Verifies NET-001..NET-004 reconnect, `/ws`, node role, and WS frame baseline contracts |
| `check-search-baseline.sh` | Verifies current Search scope, feature-gate, stale-result, and future-index boundaries |
| `check-cli-settings-baseline.sh` | Verifies CLI command surface, `config.toml` settings mutation, and shortcut entry contracts |
| `check-browser-prefs-boundary.sh` | Verifies harmless Web UI prefs are the only functional localStorage users and go through the fallback layer |
| `check-ai-baseline.sh` | Verifies Native AI slash modes, planned palette command boundaries, and trusted-cli default-off gates |
| `check-source-control-baseline.sh` | Verifies Source Control panel commit/publish boundaries and planned Git palette commands |
| `check-source-control-smoke-hygiene.sh` | Verifies Source Control smoke tests use read-only `sc-status` and do not assume Git-clean app state |
| `check-dev-data-health-baseline.sh` | Verifies projection health diagnostics expose repair hints and fail-closed authority corruption boundaries |
| `check-native-track-boundary.sh` | Verifies Desktop/Mobile native adapter boundaries remain future-safe and do not redefine core authority |
| `check-graph-baseline.sh` | Verifies Graph remains a read-only derived projection and does not become a ledger/workspace authority path |
| `check-dev-runbook-baseline.sh` | Verifies current startup, auth, frontend, Chrome MCP, search, and verification runbook boundaries |
| `check-release-baseline.sh` | Verifies Docker, compose, and release workflow surfaces match the embedded-frontend release baseline |
| `smoke-docker-release.sh` | Builds and runs the Docker release image smoke test when Docker is available |
| `smoke-runtime-release-info.sh` | Checks a running server's `/api/node/role` runtime release info fields |

## For AI Agents

### Working In This Directory

- Scripts target Windows (CMD/PowerShell) since primary dev environment is WSL on Windows.
- Keep scripts minimal and focused on a single task.

<!-- MANUAL: -->

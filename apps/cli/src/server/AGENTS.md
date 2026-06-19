<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# server

## Purpose
The main Axum HTTP/WebSocket server runtime. Manages `AppState` (RepoManager, SyncManager, broadcast channel, plugin runtime, sync engine, tree registry, identity key), builds the router with auth middleware, and handles all client connections. In-crate integration tests (scope binding, switcher, sync, source control, ws acceptance, …) live under `tests/<family>/`, declared flat via `#[path]` in `test_modules.rs`.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | `AppState` struct definition, bound-listener server startup and `start_plugin_host_only()` entry points |
| `router.rs` | Axum router construction: protected routes (JWT), public routes, login, static fallback |
| `setup.rs` | Server init helpers: CORS config, file watcher spawn, port hint writing |
| `start/tests.rs` | Server startup helper tests |
| `session/mod.rs` | Per-WebSocket-connection session state (`WsSession`): scope nonce, branch, repo, rate limiting |
| `session/` | Session helper modules for writer identity, repo binding, branch/scope binding, and rate limiting |
| `channel/mod.rs` | `DualChannel` (broadcast + unicast) with delivery classification (must-deliver vs droppable) |
| `repo_scope/mod.rs` | Repo scope facade: maps session state to active repo/branch with fail-closed validation |
| `repo_scope/` | Repo scope implementation helpers for bootstrap, cleanup, error mapping, lookup, remote resolution, selector logic, sync, and workspace paths |
| `shadow_scope.rs` | Shadow branch scope management and stale branch cleanup |
| `error_classify.rs` | Error string classification into semantic error codes |
| `error_classify/tests.rs` | Shared error classification tests |
| `tree_state.rs` | Repo-scoped file tree state registry (`RepoTreeRegistry`) |
| `tree_state/tests.rs` | Repo-scoped tree registry tests |
| `security.rs` | Identity key loading/generation |
| `rate_limit.rs` | Per-IP sliding window rate limiter with lazy GC; fails closed on poisoned lock |
| `metrics.rs` | System metrics collection and periodic broadcasting |
| `static_files.rs` | Static file serving for web frontend SPA |
| `static_files/tests.rs` | Static file serving tests |
| `prewarm.rs` | Background repo prewarm on startup |
| `node_role.rs` | Node role state (main/proxy) |
| `node_role_http.rs` | Node role HTTP endpoint |
| `notegit.rs` | Host directory preparation (.notegit, host keys) |
| `source_control_proxy/` | Source control proxy facade and implementation modules for `RemoteSourceControlApi` |
| `source_control_proxy/http/tests/` | Source control proxy HTTP error decoding tests |
| `plugin_host/mod.rs` | Plugin host server mode for satellite processes |
| `plugin_host/` | Plugin host HTTP routes and WebSocket handler |
| `plugin_response.rs` | Plugin response formatting helpers |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `handlers/` | Client message handlers organized by domain |
| `repo_scope/` | Repo scope facade and implementation modules |
| `source_control_proxy/` | Remote Source Control bridge implementation |
| `ws/` | WebSocket connection lifecycle, message routing, and broadcast filtering |
| `auth/` | Authentication middleware, JWT cookie handling, brute-force protection |
| `ai_chat/` | OpenAI-compatible streaming chat integration |
| `agent_bridge/` | Default-off Trusted CLI bridge; policy-gated, not MCP and not a generic plugin authority |
| `channel/tests/` | Integration tests for channel delivery guarantees |
| `tests/` | In-crate `#[cfg(test)]` integration tests grouped by feature family (`docs/`, `document/`, `edit/`, `key_exchange/`, `listing/`, `open_doc/`, `repo_scope/`, `source_control/`, `switcher/`, `sync/`, `ws_acceptance/`). Declared flat as children of `server` via `#[path]` in `test_modules.rs`, so test bodies reach the code under test through `super::`. |

## For AI Agents

### Working In This Directory
- **Scope nonces are critical**: Every repo-scoped message must carry the current `scope_nonce` so the frontend can discard stale messages from a previous branch/repo context. Use `session.scope_nonce()` for browser sessions.
- **Fail-closed**: Never mask a corrupted/stale scope as "no scope" or "empty". Return explicit `ServerError` with the appropriate `ServerErrorCode`.
- **DualChannel delivery**: Protocol errors, scope switches, key messages, and sync control messages are classified as must-deliver and will be async-queued if the unicast channel is full. Regular messages (Pong, metrics) are dropped.
- **Rate limiting**: Both per-IP HTTP rate limiting (`rate_limit.rs`) and per-connection WS rate limiting (`WsSession::record_incoming_message`) are enforced. Both fail closed on poisoned locks.
- **Test files** (`*_test.rs`, `*_test_support.rs`) are `#[cfg(test)]` modules grouped under `tests/<family>/` and declared flat as `server` children via `#[path]` in `test_modules.rs` — so a test body still uses `super::Foo` (not `super::super::`) to reach the code under test. When adding a test, drop the file in the right family dir and add a `#[path]` + `mod` line to `test_modules.rs`.

<!-- MANUAL: -->

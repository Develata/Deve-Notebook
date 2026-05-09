<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# server

## Purpose
The main Axum HTTP/WebSocket server runtime. Manages `AppState` (RepoManager, SyncManager, broadcast channel, plugin runtime, sync engine, tree registry, identity key), builds the router with auth middleware, and handles all client connections. Contains ~40+ integration test files for scope binding, switcher, sync, and source control behavior.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | `AppState` struct definition, `start_server()` and `start_plugin_host_only()` entry points |
| `router.rs` | Axum router construction: protected routes (JWT), public routes, login, static fallback |
| `setup.rs` | Server init helpers: CORS config, file watcher spawn, port hint writing |
| `start/tests.rs` | Server startup helper tests |
| `session/mod.rs` | Per-WebSocket-connection session state (`WsSession`): scope nonce, branch, repo, rate limiting |
| `session/` | Session helper modules for writer identity, repo binding, branch/scope binding, and rate limiting |
| `channel/mod.rs` | `DualChannel` (broadcast + unicast) with delivery classification (must-deliver vs droppable) |
| `repo_scope/mod.rs` | Repo scope facade: maps session state to active repo/branch with fail-closed validation |
| `repo_scope/` | Repo scope implementation helpers for bootstrap, cleanup, error mapping, lookup, remote resolution, selector logic, sync, and workspace paths |
| `repo_scope_test/` | Repo scope catalog, error mapping, local scope, and alias tests |
| `repo_scope_recovery_test_extra/` | Repo scope recovery and local-counterpart tests |
| `repo_scope*_test.rs` | Remaining repo scope runtime selector and recovery tests |
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
| `list_docs_scope_test/` | Repo-scoped document listing binding tests |
| `listing_scope_binding_test/` | Listing bootstrap and stale runtime binding tests |
| `listing_scope_cleanup_test/` | Listing stale scope cleanup tests |
| `listing_shadow_scope_test/` | Shadow branch listing scope tests |
| `listing_*_test.rs` | Remaining listing integration tests |
| `source_control_proxy/` | Source control proxy facade and implementation modules for `RemoteSourceControlApi` |
| `source_control_http_*_test.rs` | Source control HTTP roundtrip tests and helpers |
| `source_control_proxy/http/tests/` | Source control proxy HTTP error decoding tests |
| `source_control_changes_identity*_test.rs` | Source control changes identity retention tests |
| `source_control_local_commit_scope*_test.rs` | Source control commit scope nonce and bootstrap tests |
| `source_control_{local,remote}_scope*_test.rs` | Source control scope/identity runtime tests |
| `source_control_scope*_test.rs` | Source control scope binding and selector runtime tests |
| `switcher_branch_test/` | Branch switcher success, reject, and scope message tests |
| `switcher_branch_scope_test/` | Branch switcher repo-scope fail-closed and selector binding tests |
| `switcher_current_scope_test/` | Current local/remote scope validation tests for branch switching |
| `switcher_exact_selector_test/` | Exact remote selector switcher collision and fail-closed tests |
| `switcher_*_test.rs` | Remaining switcher integration tests |
| `switcher_test_support.rs` | Shared switcher test harness for AppState/session/unicast setup |
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

## For AI Agents

### Working In This Directory
- **Scope nonces are critical**: Every repo-scoped message must carry the current `scope_nonce` so the frontend can discard stale messages from a previous branch/repo context. Use `session.scope_nonce()` for browser sessions.
- **Fail-closed**: Never mask a corrupted/stale scope as "no scope" or "empty". Return explicit `ServerError` with the appropriate `ServerErrorCode`.
- **DualChannel delivery**: Protocol errors, scope switches, key messages, and sync control messages are classified as must-deliver and will be async-queued if the unicast channel is full. Regular messages (Pong, metrics) are dropped.
- **Rate limiting**: Both per-IP HTTP rate limiting (`rate_limit.rs`) and per-connection WS rate limiting (`WsSession::record_incoming_message`) are enforced. Both fail closed on poisoned locks.
- **Test files** (`*_test.rs`, `*_test_support.rs`) are `#[cfg(test)]` modules; there are ~40+ scope/binding/switcher/sync tests co-located here.

<!-- MANUAL: -->

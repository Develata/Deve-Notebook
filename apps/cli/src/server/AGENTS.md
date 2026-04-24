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
| `setup.rs` | Server init helpers: CORS config, MCP loading, file watcher spawn, port hint writing |
| `session.rs` | Per-WebSocket-connection session state (`WsSession`): scope nonce, branch, repo, rate limiting |
| `session_scope.rs` | Session-level scope binding helpers (switch_branch, switch_repo, clear_active_db) |
| `channel.rs` | `DualChannel` (broadcast + unicast) with delivery classification (must-deliver vs droppable) |
| `repo_scope.rs` | Repo scope resolution: maps session state to active repo/branch with fail-closed validation |
| `repo_scope_bootstrap.rs` | Bootstrap repo scope for single-repo sessions that have no explicit selection |
| `repo_scope_cleanup.rs` | Cleanup stale repo scope bindings |
| `repo_scope_error.rs` | Repo scope error classification |
| `repo_scope_lookup.rs` | Repo scope lookup helpers |
| `repo_scope_remote.rs` | Remote repo scope resolution |
| `repo_scope_selector.rs` | Repo scope selector logic |
| `repo_scope_workspace.rs` | Workspace-level repo scope |
| `shadow_scope.rs` | Shadow branch scope management and stale branch cleanup |
| `error_classify.rs` | Error string classification into semantic error codes |
| `error_classify_test.rs` | Shared error classification tests |
| `tree_state.rs` | Repo-scoped file tree state registry (`RepoTreeRegistry`) |
| `security.rs` | Identity key loading/generation |
| `rate_limit.rs` | Per-IP sliding window rate limiter with lazy GC; fails closed on poisoned lock |
| `metrics.rs` | System metrics collection and periodic broadcasting |
| `static_files.rs` | Static file serving for web frontend SPA |
| `prewarm.rs` | Background repo prewarm on startup |
| `node_role.rs` | Node role state (main/proxy) |
| `node_role_http.rs` | Node role HTTP endpoint |
| `notegit.rs` | Host directory preparation (.notegit, host keys) |
| `list_docs_scope*_test.rs` | Repo-scoped document listing binding tests |
| `listing_shadow_scope*_test.rs` | Shadow branch listing scope tests |
| `source_control_proxy.rs` | Source control proxy: `RemoteSourceControlApi` implementing `Repository` trait |
| `source_control_proxy_client.rs` | Source control proxy HTTP client construction |
| `source_control_proxy_http.rs` | HTTP proxy for remote source control operations |
| `source_control_proxy_http_plain.rs` | Plain-text proxy error classification |
| `source_control_proxy_http_target.rs` | Remote SC target-specific error mapping |
| `source_control_proxy_commits.rs` | Commit history proxy operations |
| `source_control_proxy_mutations.rs` | Mutation proxy (stage/unstage/discard/commit) |
| `source_control_proxy_queries.rs` | Query proxy (changes/diff/pending) |
| `source_control_http_*_test.rs` | Source control HTTP roundtrip tests and helpers |
| `source_control_proxy_http_*_test.rs` | Source control proxy HTTP error decoding tests |
| `source_control_changes_identity*_test.rs` | Source control changes identity retention tests |
| `source_control_local_commit_scope*_test.rs` | Source control commit scope nonce and bootstrap tests |
| `source_control_{local,remote}_scope*_test.rs` | Source control scope/identity runtime tests |
| `source_control_scope*_test.rs` | Source control scope binding and selector runtime tests |
| `plugin_host.rs` | Plugin host server mode for satellite processes |
| `plugin_host_routes.rs` | Plugin host HTTP routes |
| `plugin_host_ws.rs` | Plugin host WebSocket handler |
| `plugin_response.rs` | Plugin response formatting helpers |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `handlers/` | Client message handlers organized by domain |
| `ws/` | WebSocket connection lifecycle, message routing, and broadcast filtering |
| `auth/` | Authentication middleware, JWT cookie handling, brute-force protection |
| `ai_chat/` | OpenAI-compatible streaming chat integration |
| `agent_bridge/` | Bridge to external AI CLI tools |
| `channel_test/` | Integration tests for channel delivery guarantees |
| `mcp/` | Model Context Protocol client executors (stdio, HTTP, SSE) |

## For AI Agents

### Working In This Directory
- **Scope nonces are critical**: Every repo-scoped message must carry the current `scope_nonce` so the frontend can discard stale messages from a previous branch/repo context. Use `session.scope_nonce()` for browser sessions.
- **Fail-closed**: Never mask a corrupted/stale scope as "no scope" or "empty". Return explicit `ServerError` with the appropriate `ServerErrorCode`.
- **DualChannel delivery**: Protocol errors, scope switches, key messages, and sync control messages are classified as must-deliver and will be async-queued if the unicast channel is full. Regular messages (Pong, metrics) are dropped.
- **Rate limiting**: Both per-IP HTTP rate limiting (`rate_limit.rs`) and per-connection WS rate limiting (`WsSession::record_incoming_message`) are enforced. Both fail closed on poisoned locks.
- **Test files** (`*_test.rs`, `*_test_support.rs`) are `#[cfg(test)]` modules; there are ~40+ scope/binding/switcher/sync tests co-located here.

<!-- MANUAL: -->

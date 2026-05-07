<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# use_core

## Purpose

Core application state management hub. The largest hook module — manages WebSocket message processing, source control state, repo/branch switching, sync state, dashboard context, diff sessions, navigation, and all server message effects.

## Key Files

| File | Description |
|------|-------------|
| `mod.rs` | use_core hook entry and composition |
| `state.rs` | Core reactive state signals |
| `state_build.rs` | State construction from server data |
| `state_init.rs` | Initial state setup |
| `provide.rs` | Context provider for child components |
| `contexts.rs` | Context type definitions |
| `types.rs` | Shared type definitions |
| `callbacks.rs` | General callback handlers |
| `callbacks_sc.rs` | Source control callbacks |
| `callbacks_sc_scope.rs` | SC scope callbacks |
| `callbacks_sc_target.rs` | SC target callbacks |
| `callbacks_scope.rs` | Scope-related callbacks |
| `callbacks_switch.rs` | Repo/branch switch callbacks |
| `callbacks_sync.rs` | Sync callbacks |
| `apply.rs` | State application logic |
| `navigation.rs` | Document navigation |
| `dashboard_context.rs` | Dashboard data context |
| `diff_session.rs` | Diff session management |
| `effects_sc_test_read_lists.rs` | Source-control read-list test harness |
| `effects_sc_test_read_lists_changes.rs` | ChangesList scope dispatch tests |
| `effects_sc_test_read_lists_history.rs` | CommitHistory scope dispatch tests |
| `storage_runtime.rs` | Storage runtime integration |
| `switch_nonce.rs` | Switch nonce tracking |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `effects/` | Server message effects (handshake, dispatch, protocol) |
| `pending/` | Pending operation tracking |

## For AI Agents

### Working In This Directory

- This is the state management core — changes here affect the entire UI.
- `effects/` processes incoming ServerMessages and updates reactive signals.
- Scope nonce validation happens in `switch_nonce.rs` and `callbacks_scope.rs`.
- `switch_nonce.rs` must always generate a nonce strictly greater than the current scope nonce; browser switch requests fail closed on stale values.
- `scope_prefs.rs` persists stable `repo_name + repo_id + active_branch` and should restore scope using UUID-first semantics.
- Session/auth polling should pause while the page is backgrounded and resume when the document becomes active again.

<!-- MANUAL: -->

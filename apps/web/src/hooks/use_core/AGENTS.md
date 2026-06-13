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
| `state/` | Core signal and plugin response type slices |
| `state_build.rs` | State construction from server data |
| `state_build/` | CoreState assembly sections |
| `state_init.rs` | Initial state setup |
| `state_init/` | Initial signal factories and initialization submodules |
| `provide.rs` | Context provider for child components |
| `provide/` | Context provider builders by child context |
| `contexts.rs` | Context type definitions |
| `contexts/` | Context type slices by UI domain |
| `types.rs` | Shared type definitions |
| `types/` | CoreState, chat helpers, and shared signal structs |
| `callbacks.rs` | General callback handlers |
| `callbacks/` | General callback domain slices for docs, write paths, search, plugin, and stats |
| `callbacks_build/` | Core callback assembly helpers |
| `callbacks_sc/` | Source-control read/write callback slices |
| `callbacks_sync/` | Sync callback read/write helpers |
| `callbacks_sc.rs` | Source control callbacks |
| `callbacks_sc_scope.rs` | SC scope callbacks |
| `callbacks_sc_scope/` | SC scope callback tests |
| `callbacks_sc_target.rs` | SC target callbacks |
| `callbacks_sc_target/` | SC target callback tests |
| `callbacks_scope.rs` | Scope-related callbacks |
| `callbacks_switch.rs` | Repo/branch switch callbacks |
| `callbacks_switch/` | Repo/branch switch callback slices |
| `callbacks_sync.rs` | Sync callbacks |
| `apply.rs` | State application logic |
| `apply/` | Tree lookup and node mutation helpers |
| `navigation.rs` | Document navigation |
| `dashboard_context.rs` | Dashboard data context |
| `diff_session.rs` | Diff session management |
| `effects_sc.rs` | Source-control message dispatch facade |
| `effects_sc/` | Source-control message dispatch slices and tests |
| `effects_sc_apply.rs` | Source-control refresh/apply facade |
| `effects_sc_apply/` | Source-control refresh/apply slices and tests |
| `effects_sc_feedback/` | Source-control feedback tests |
| `effects_switch/` | Repo/branch switch effect slices and tests |
| `storage_runtime.rs` | Storage runtime integration |
| `storage_runtime/` | Storage bootstrap/effect/repo helpers |
| `switch_nonce.rs` | Switch nonce tracking |
| `write_gate.rs` | Repo write/read gate facade |
| `write_gate/` | Repo gate logic and tests |
| `status_summary.rs` | Sync status summary derivation |
| `status_summary/` | Sync status summary tests |
| `scope_prefs.rs` | Last repo scope preference persistence |
| `scope_prefs/` | Scope preference tests |

## Subdirectories

| Directory | Purpose |
|-----------|---------|
| `effects/` | Server message effects (handshake, dispatch, protocol) |

## For AI Agents

### Working In This Directory

- This is the state management core — changes here affect the entire UI.
- `effects/` processes incoming ServerMessages and updates reactive signals.
- Scope nonce validation happens in `switch_nonce.rs` and `callbacks_scope.rs`.
- `switch_nonce.rs` must always generate a nonce strictly greater than the current scope nonce; browser switch requests fail closed on stale values.
- `scope_prefs.rs` persists only the last `repo_name` display alias for server-side re-resolution; it must not store `repo_id`, active branch / peer id, `scope_nonce`, or any repo authority identity.
- Session/auth polling should pause while the page is backgrounded and resume when the document becomes active again.

<!-- MANUAL: -->

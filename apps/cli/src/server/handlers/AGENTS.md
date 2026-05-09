<!-- Parent: ../AGENTS.md -->
<!-- Generated: 2026-03-22 -->

# handlers

## Purpose
Client message handlers organized by domain. Each submodule processes a category of `ClientMessage` variants received over WebSocket or HTTP and produces `ServerMessage` responses via the `DualChannel`. This is where the server's business logic lives at the transport boundary.

## Key Files
| File | Description |
|------|-------------|
| `mod.rs` | Module declarations for all handler domains |
| `admin.rs` | Admin HTTP endpoints: dump, export, node-check with error classification |
| `admin_dump.rs` | Admin debug dump endpoint implementation |
| `admin_test.rs` | Admin handler tests |
| `listing/` | ListDocs, ListShadows, ListRepos handlers with stale scope precheck and cleanup |
| `listing/docs.rs` | Document listing implementation with projection refresh |
| `switcher/` | Branch/repo switching orchestration: validates targets, prepares scope, commits switch |
| `switcher/switcher_scope.rs` | Resolves current branch switch context for scope transitions |
| `switcher/switcher_selector.rs` | Target repo selection: exact stem match, UUID lookup, URL recovery |
| `switcher/switcher_prepare.rs` | Prepares repo switch: resolves DB handle, repo_id, validates metadata |
| `switcher/switcher_guard.rs` | Guards switch operations requiring browser session and switch_nonce |
| `switcher/switcher_payload.rs` | Builds and emits the DocList + TreeUpdate + RepoSwitched message sequence |
| `switcher/switcher_error.rs` | Switcher-specific error construction |
| `document.rs` | Document content ops entry point (delegates to `document/` submodule) |
| `merge.rs` | Merge operations entry point (delegates to `merge/` submodule) |
| `sync.rs` | Sync operations entry point (delegates to `sync/` submodule) |
| `key_exchange.rs` | E2EE key exchange: provides RepoKey over authenticated WebSocket |
| `search.rs` | Full-text search handler (feature-gated behind `search`) |
| `plugin.rs` | Plugin RPC handler: routes to Rhai plugins or agent-bridge |

## Subdirectories
| Directory | Purpose |
|-----------|---------|
| `docs/` | Document CRUD operations (create, rename, delete, copy) |
| `docs/copy/` | Directory and file copy implementation |
| `document/` | Document content operations (open, edit, snapshot, history) |
| `merge/` | Merge conflict resolution and manual sync mode handlers |
| `repo/` | Repository management HTTP endpoints |
| `source_control/` | Git-like source control (changes, staging, commits, diff) |
| `source_control/errors/` | Source control error mapping |
| `source_control/service/` | Source control service layer (read/write/target resolution) |
| `switcher/switcher_prepare_test/` | Switcher preparation integration tests |
| `sync/` | P2P sync engine handlers (hello, transfer, snapshot, cleanup) |

## For AI Agents

### Working In This Directory
- Every handler must respect repo scope; use `resolve_session_repo_and_sync` or `bootstrap_local_repo` from `repo_scope/mod.rs`.
- Listing handlers precheck for stale remote unbound scopes before querying data.
- Switcher is a multi-phase pipeline: guard -> validate target -> select repo -> prepare -> preload view -> commit session -> emit messages.
- Branch switching now carries a session-level "last local repo" hint so `remote -> Local` returns to the user's previous local repo when possible.
- Target repo selection stays fail-closed by default, but may fall back to the single available remote repo when a remote target set is unambiguous.
- Admin handlers classify errors to HTTP status codes via `error_classify.rs` patterns.
- Plugin handler intercepts `agent-bridge` plugin_id and routes to the agent bridge instead of Rhai; bundled AI modes expose only `chat` as public RPC.

<!-- MANUAL: -->

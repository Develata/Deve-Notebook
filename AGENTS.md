# Developer & Agent Guidelines (Math-Architect Edition)

This repository is a Rust workspace designed for high-performance, low-resource environments (target: 768MB RAM VPS). It comprises a core library (`crates/core`), a web frontend (`apps/web` using Leptos), and a CLI (`apps/cli`).

## 1. Project Architecture & Constraints

- **Core Philosophy**: Minimal resource usage, mathematical rigor, and strict modularity.
- **Target Environment**: **768MB - 1GB RAM VPS**. Avoid heavy dependencies. Prefer Zero-copy implementations.
- **Structure**:
  - **`crates/core`**: Business logic, storage (Redb), Sync (CRDT), Search (Tantivy), Plugins (Rhai).
  - **`apps/web`**: WASM Frontend (Leptos).
  - **`apps/cli`**: Axum-based Server & CLI.
  - **`plugins`**: Built-in Rhai plugins (e.g. `ai-chat`).

## 2. Strict Engineering Standards (The "Iron Rules")

### File Size Limits (Crucial)
To maintain maintainability and cognitive load control:
- **Target**: **< 130 lines** per file.
- **Hard Limit (Circuit Breaker)**: **250 lines**. 
  - *Action*: If you need to write line 251, you **must** refactor and split the module immediately.

### Code Style & Safety
- **Language**: Rust (Edition 2024).
- **Formatting**: strict `cargo fmt`.
- **Linting**: `cargo clippy --all-targets --all-features -- -D warnings`.
- **Invariants**: For complex logic (Sync, Graph, Storage), you **must** document "Invariants", "Pre-conditions", and "Post-conditions" in doc comments. This prepares the code for future formal verification (Lean4).
- **Error Handling**:
  - App: `anyhow::Result`.
  - Lib: `thiserror` (Recoverable).
- **Path Handling**: Windows-compatible (`std::path::Path`), use `deve_core::utils::path::to_forward_slash`.

## 3. Build & Test Commands

### General
- **Build All**: `cargo build --release` (Check memory usage)
- **Test All**: `cargo test`
- **Lint**: `cargo clippy`

### Efficient Testing (Single Test)
Do not run the full suite repeatedly. Target specific tests:

```bash
# Template
cargo test --package <package_name> --lib <test_function_name> -- --nocapture

# Example: Run 'test_merge_conflict' in core
cargo test --package deve_core --lib test_merge_conflict -- --nocapture

# Example: Run plugin system tests
cargo test --package deve_core --test plugin_test -- --nocapture
```

### Frontend (Leptos)
Requires `trunk` and `npm`.

1.  **Setup**: `cargo install trunk` & `cd apps/web && npm install`.
2.  **Dev Server**: `trunk serve` (runs on 127.0.0.1:8080).
    *   *Note*: Ensure `deve_cli serve` is running for backend API support if needed, though trunk proxies calls.

## 4. Agent Workflow Protocol

1.  **Docs First**: Check `deve-note plan/`, `deve-note report/schedules`, or `README.md` before coding.
2.  **Low-Resource Assessment**: 
    - Before adding a dependency, ask: "Will this run on 768MB RAM?"
    - If `false`, find a lighter alternative or implement a minimal version.
3.  **Math/Logic Verification**:
    - When implementing sync/storage logic, explicitly state the algorithm's invariants.
    - Example: "Invariant: The Lamport timestamp must strictly increase per actor."
4.  **Edit Loop**:
    - **Read** file.
    - **Plan** change (checking line count limit).
    - **Edit/Write**.
    - **Verify**: Run specific test + `cargo clippy`.

## 5. Directory Map

- `crates/core/src/ledger`: Append-only log & storage (Redb).
- `crates/core/src/sync`: Synchronization, Conflict resolution, Vector Clock.
- `crates/core/src/plugin`: Rhai script runtime & Host API.
- `crates/core/src/context`: Context Engine (Directory Tree, etc.).
- `apps/cli/src/server`: WebSocket sync server (Axum).
- `apps/web/src/components`: Leptos UI components (Chat, Editor, Sidebar).

## 6. Commit Convention

- `feat`: New features (check resource impact).
- `fix`: Bug fixes.
- `refactor`: Splitting files > 130 lines.
- `proof`: Adding formal comments or Lean4 definitions.

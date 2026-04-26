# cli_server_runtime.md - CLI Server Runtime 链

## Metadata

- `Flow ID`: `flow.cli.server-runtime`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`, `docs/features/15_release.md`
- `Related Acceptance Cases`: `CMD-001`, `CMD-007`, `REL-002`

## Operations

### `op.cli.server.choose-port`

- `Name`: `Choose Server Port`
- `Surface`: `cli`
- `Trigger`: run `deve serve --port <port>`
- `Preconditions`: requested port is valid
- `Immediate Result`: server runtime receives the selected bind port
- `Application Entry`: `apps/cli/src/main.rs`, `apps/cli/src/commands/serve.rs`

### `op.cli.server.enable-dev-mode`

- `Name`: `Enable Dev Mode`
- `Surface`: `cli`
- `Trigger`: run `deve serve --dev`
- `Preconditions`: development mode is intentionally requested
- `Immediate Result`: server startup uses development-mode runtime flags; missing `AUTH_SECRET` / `AUTH_PASS` falls back to explicit dev defaults only for this mode
- `Application Entry`: `apps/cli/src/commands/serve.rs`

### `op.cli.server.dry-run`

- `Name`: `Dry Run Server Startup`
- `Surface`: `cli`
- `Trigger`: run `deve serve --dry-run`
- `Preconditions`: config is readable
- `Immediate Result`: server startup validates configuration without binding
- `Application Entry`: `apps/cli/src/commands/serve.rs`

### `op.cli.server.start`

- `Name`: `Start Local Hub`
- `Surface`: `cli`
- `Trigger`: run `deve serve`
- `Preconditions`: ledger, vault, and server config are valid
- `Immediate Result`: Axum HTTP/WebSocket server starts
- `Application Entry`: `apps/cli/src/commands/serve.rs`, `apps/cli/src/server/`

## Response Flow

1. User chooses server startup options.
2. Instruction interface parses `Commands::Serve`.
3. Flow coordination calls the serve command and server bootstrap.
4. Execution domains are config, protocol, sync, ledger, and local hub runtime.

## Notes

- `--dry-run` is modeled as a distinct operation because it changes side effects.
- Local UI verification can use the embedded/bundled frontend only when the CLI was built after the latest `trunk build --release`; otherwise it may serve stale WASM. Use the two-process flow as a fallback: backend `deve serve --dev --port 3001`, then `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080` from `apps/web`.
- Auth smoke must open `http://127.0.0.1:<port>/`. Do not use `0.0.0.0` as the browser origin because secure cookie behavior is origin-sensitive in local HTTP testing.
- Backend-only `deve serve --dev` may return 404 on `/` only when neither embedded assets nor a valid `DEVE_STATIC_DIR` are available.
- Main objects: `server::bind`, `config::runtime`, `repo::scope`, `cli::option`.

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
- `Preconditions`: config and host registries are readable；zero `Normal` catalog members is valid
- `Immediate Result`: server startup validates the existing cataloged runtime without binding or creating a local repo database；empty catalog reports healthy `NoScope`
- `Failure Result`: corrupt host registry or typed host-fatal fails closed；an invalid existing repo is reported as repo-local readonly/repair state rather than fabricating a default repo
- `Application Entry`: `apps/cli/src/commands/serve.rs`

### `op.cli.server.start`

- `Name`: `Start Local Hub`
- `Surface`: `cli`
- `Trigger`: run `deve serve`
- `Preconditions`: ledger host registries and server config are valid；each existing local repo is independently admitted when its authority identity and Projection Locator/workspace are valid
- `Immediate Result`: Axum HTTP/WebSocket server starts after host runtime composition; zero-repo is healthy `BootstrapUnbound(scope_nonce=0)` without writer readiness, while existing repo watcher failures remain typed readonly state
- `Partial Result`: repo-local watcher start failure leaves that repo readonly/diagnostic while other Mounted repos remain writable
- `Failure Result`: only a typed host-fatal closes all started handles and exits non-zero；zero repo or zero Mounted repo-local outcomes remain a running readonly/diagnostic/Create-capable host
- `Application Entry`: `apps/cli/src/commands/serve.rs`, `apps/cli/src/server/`

## Response Flow

1. User chooses server startup options.
2. Instruction interface parses `Commands::Serve`.
3. Flow coordination calls the serve command and server bootstrap.
4. Execution domains are config, protocol, sync, ledger, local hub runtime, and host-owned watcher supervision; handlers receive only readonly `WatcherRuntimeView`.

## Notes

- `--dry-run` is modeled as a distinct operation because it changes side effects.
- `deve serve` and `deve serve --dry-run` never use generic create-capable repo initialization. Repo creation belongs to `deve init` or the typed repo lifecycle coordinator.
- After successful bootstrap, later failure of every watcher keeps the process alive for readonly/diagnostic access and reports aggregate health as degraded; it does not reclassify repo-local failure as host-fatal.
- Fresh UI smoke data roots must first run `deve init --path <data-root> --repo default --projection-base <projection-base>` or otherwise point `DEVE_LEDGER_DIR` at the existing ledger directory for a data root with a valid host-local Projection Locator.
- Local UI verification can use the embedded/bundled frontend only when the CLI was built after the latest `trunk build --release`; otherwise it may serve stale WASM. Use the two-process flow as a fallback after locator prep: backend `deve serve --dev --port 3001`, then `NO_COLOR=true trunk serve --address 127.0.0.1 --port 8080` from `apps/web`.
- Auth smoke must open `http://127.0.0.1:<port>/`. Do not use `0.0.0.0` as the browser origin because secure cookie behavior is origin-sensitive in local HTTP testing.
- Backend-only `deve serve --dev` may return 404 on `/` only when neither embedded assets nor a valid `DEVE_STATIC_DIR` are available.
- Main objects: `server::bind`, `config::runtime`, `repo::scope`, `cli::option`.

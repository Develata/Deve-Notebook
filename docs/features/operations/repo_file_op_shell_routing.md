# repo_file_op_shell_routing.md - File Operation 命令壳路由链

## Metadata

- `Flow ID`: `flow.repo.file-op-shell-routing`
- `Domain`: `repository`
- `Related Feature Chapters`: `docs/features/06_repository.md`, `docs/features/08_ui_design.md`, `docs/features/12_commands.md`
- `Related Acceptance Cases`: `REPO-FEAT-01`, `UI-DESK-003`

## Operations

### `op.repo.file-shell.enter-command`

- `Name`: `Enter File Operation Command`
- `Surface`: `search-box`
- `Trigger`: type `>mv`, `>cp`, or `>rm`
- `Preconditions`: repo-scoped search surface is open
- `Immediate Result`: query enters file-op command mode
- `Application Entry`: `apps/web/src/components/search_box/file_ops/mod.rs`, `apps/web/src/components/search_box/logic/providers.rs`

### `op.repo.file-shell.parse-query`

- `Name`: `Parse File Operation Query`
- `Surface`: `search-input`
- `Trigger`: continue typing file-op arguments
- `Preconditions`: file-op command mode is active
- `Immediate Result`: args, quote state, and destination readiness are parsed
- `Application Entry`: `apps/web/src/components/search_box/file_ops/parser.rs`

### `op.repo.file-shell.prefill-destination`

- `Name`: `Prefill File Operation Destination`
- `Surface`: `search-results`
- `Trigger`: choose a suggested move/copy destination
- `Preconditions`: parsed command is eligible for destination completion
- `Immediate Result`: `SearchAction::InsertQuery` rewrites the query with destination text and cursor position
- `Application Entry`: `apps/web/src/components/search_box/file_ops/results_common.rs`, `apps/web/src/components/search_box/file_ops/results_move_copy.rs`

### `op.repo.file-shell.build-action`

- `Name`: `Build File Operation Action`
- `Surface`: `search-results`
- `Trigger`: parsed command has enough source and destination data to execute
- `Preconditions`: normalized path intent is valid for the current file-op kind
- `Immediate Result`: `SearchAction::FileOp` becomes the downstream dispatch target
- `Application Entry`: `apps/web/src/components/search_box/file_ops/results_common.rs`, `apps/web/src/components/search_box/file_ops/results_remove.rs`

## Response Flow

1. User types a file-op command into the shared search surface.
2. Instruction interface detects file-op mode and parses command arguments.
3. Flow coordination either pre-fills the next query stage or builds a concrete `FileOpAction`.
4. Execution domains touched here are command routing and tree-aware path intent shaping; actual repo writes continue in `repo_file_operations.md`.

## Notes

- This flow is the shared shell before the concrete write flow in `repo_file_operations.md`.
- It explains how `CreateDoc`, `FileOp`, and `InsertQuery` stay on one command surface instead of branching into ad hoc UI paths.
- Main objects: `search::action`, `fileop::intent`, `command::registry`.

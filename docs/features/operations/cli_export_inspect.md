# cli_export_inspect.md - CLI 导出与检查链

## Metadata

- `Flow ID`: `flow.cli.export-inspect`
- `Domain`: `commands`
- `Related Feature Chapters`: `docs/features/12_commands.md`, `docs/features/14_tech_stack.md`
- `Related Acceptance Cases`: `CMD-001`, `CMD-008`, `TECH-002`

## Operations

### `op.cli.inspect.dump-path`

- `Name`: `Dump File Ops`
- `Surface`: `cli`
- `Trigger`: run `deve dump --path <path> [--repo <repo>]`
- `Preconditions`: path is provided and repo scope can be resolved
- `Immediate Result`: debug dump command selects ledger entries for the path
- `Application Entry`: `apps/cli/src/commands/dump.rs`

### `op.cli.export.choose-format`

- `Name`: `Choose Export Format`
- `Surface`: `cli`
- `Trigger`: run `deve export --format <json|markdown>`
- `Preconditions`: format value is supported
- `Immediate Result`: export renderer selects JSONL or Markdown output path; degraded projection Markdown export requires explicit `--allow-degraded-projection`
- `Application Entry`: `apps/cli/src/commands/export.rs`

### `op.cli.export.select-doc`

- `Name`: `Select Export Document`
- `Surface`: `cli`
- `Trigger`: run `deve export --doc <doc_id>`
- `Preconditions`: doc id exists inside resolved repo scope
- `Immediate Result`: export narrows from repository scope to one document
- `Application Entry`: `apps/cli/src/commands/export.rs`

### `op.cli.export.choose-output`

- `Name`: `Choose Export Output`
- `Surface`: `cli`
- `Trigger`: run `deve export --output <path>`
- `Preconditions`: output path is writable
- `Immediate Result`: export artifact destination is fixed
- `Application Entry`: `apps/cli/src/commands/export.rs`

## Response Flow

1. User chooses dump or export options.
2. Instruction interface parses `Commands::{Dump, Export}`.
3. Flow coordination resolves repo/doc scope and output settings.
4. Execution domains are ledger, tree projection, export rendering, and filesystem.

## Notes

- This flow separates inspection from mutation; it should not change ledger state.
- Main objects: `repo::scope`, `doc::content`, `export::artifact`, `cli::option`.

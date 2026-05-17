# Foundation Acceptance Closure - 2026-05-17

本报告记录 `TERM-001..003` 与 `POS-001..006` 的自动守卫闭合。`docs/plan/` 未修改。

## Scope

- `TERM-001..003`: normative language, core terminology, and Ledger source-of-truth wording remain explicit.
- `POS-001`: `deve init` creates the Vault / local ledger / remote ledger layout.
- `POS-002`: external filesystem edits enter `pending_fs_ops` and do not append ledger facts directly.
- `POS-003`: Deve projection writeback does not loop back into watcher pending state.
- `POS-004`: watcher rename pairing preserves `DocId`.
- `POS-005`: `.deveignore` applies to watcher events and startup scan.
- `POS-006`: heavyweight defaults remain outside the core path.

## Changes

- Added `scripts/check-foundation-baseline.sh`.
- Registered the new guard in `docs/dev-runbook.md` and `scripts/AGENTS.md`.
- Bound foundation acceptance cases to existing plan text and existing init/watcher/rename/deveignore tests.
- No runtime behavior was changed.

## Verification

Ran:

- `bash scripts/check-foundation-baseline.sh`
- `bash scripts/check-dev-runbook-baseline.sh`
- `cargo test -p deve_cli init_creates_trinity_workspace_layout -- --nocapture`
- `cargo test -p deve_core watcher_records_create_modify_delete_candidates -- --nocapture`
- `cargo test -p deve_core projection_writeback_events_are_suppressed -- --nocapture`
- `cargo test -p deve_core watcher_pairs_rename_and_preserves_doc_identity -- --nocapture`
- `cargo test -p deve_core --test watcher_internal_ignore -- --nocapture`
- `bash scripts/check-acceptance-bindings.sh`

Results:

- Foundation baseline: passed.
- Targeted tests: passed.
- Acceptance bindings after this batch: automated `146`, feature walkthrough `54`, manual `0`, unbound `0`.

## Decision

Foundation acceptance residue is closed. Remaining acceptance cases are now either automated or feature-walkthrough bound.

Next batch: **Post-Acceptance Mainline Gap Rescan**.

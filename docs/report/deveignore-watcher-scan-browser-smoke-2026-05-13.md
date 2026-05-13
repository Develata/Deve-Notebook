# .deveignore Watcher / Scan Browser Smoke

Date: 2026-05-13

## Scope

- Plan source: `docs/plan/04_storage.md` watcher contract and ignore filtering.
- Acceptance binding: `docs/acceptance-cases/02_positioning.md` `POS-004`, `docs/acceptance-cases/07_storage_repo.md` `STORE-007`.
- Runtime surface: Web Source Control, startup scan, watcher incremental ingestion, repo docs API, graph API, export/dump CLI.
- Data root: isolated temp root `/tmp/deve-ignore-smoke-20260513-GcVCBG`.
- Server: `DEVE_LEDGER_DIR=/tmp/deve-ignore-smoke-20260513-GcVCBG/ledger DEVE_VAULT_PATH=/tmp/deve-ignore-smoke-20260513-GcVCBG/vault cargo run -p deve_cli --bin deve_cli -- serve --dev --port 32118`.

## Ignore Rules

Vault root `.deveignore`:

```text
ignored/*.md
default/rootignored/*.md
```

This covers both repo-relative and vault-relative matching.

## Automated Guards

- `bash scripts/check-storage-repo-baseline.sh` -> passed
- `cargo test -p deve_core watcher_records_create_modify_delete_candidates -- --nocapture` -> passed
- `cargo test -p deve_core watcher_respects_deveignore_for_matching_markdown -- --nocapture` -> passed
- `cargo test -p deve_core watcher_startup_scan_respects_deveignore -- --nocapture` -> passed

## Startup Scan

Files present before server startup:

- `default/ignored/startup-ignored.md`
- `default/rootignored/startup-rootignored.md`
- `default/visible/startup-visible.md`

Observed server startup scan:

- `SyncScan: Repo default 磁盘上发现 1 个 md 文件`
- Source Control showed only `visible/startup-visible.md` as `Added`.
- Browser body did not show `startup-ignored.md` or `startup-rootignored.md`.
- `/api/sc/pending` returned only `visible/startup-visible.md`.
- `/api/sc/status` returned only `visible/startup-visible.md`.
- `/api/repo/docs` returned `[]`.
- `/api/repo/graph` returned empty `nodes`, `edges`, and `unresolved_links`.
- `/api/admin/export` did not contain ignored paths.

## Watcher Incremental Path

Files created while the server and watcher were running:

- `default/ignored/later-ignored.md`
- `default/rootignored/later-rootignored.md`
- `default/visible/later-visible.md`

Observed runtime state:

- Source Control showed exactly two pending entries:
  - `visible/startup-visible.md`
  - `visible/later-visible.md`
- `/api/sc/pending` returned exactly those two visible paths.
- `/api/sc/status` returned exactly those two visible paths.
- Browser body, pending/status JSON, docs JSON, graph JSON, and export output did not contain `later-ignored.md`, `startup-ignored.md`, `later-rootignored.md`, or `startup-rootignored.md`.
- Server log recorded `Handler: New file detected: visible/later-visible.md` only for the visible incremental file.

## Reload Recovery

After page reload:

- `/api/sc/pending` still returned only visible paths.
- `/api/repo/docs` remained empty.
- `/api/repo/graph` remained empty.
- No disconnected overlay was visible.
- Current navigation console `error` / `warn` list was empty.
- Current navigation document/fetch requests returned 200.

## Ledger Check

After stopping the server:

- `deve_cli export --format json` emitted no ledger document facts.
- `deve_cli dump --path ignored/startup-ignored.md` printed `Path not found in Ledger.`

## Result

`.deveignore` filtering is browser-smoke verified for startup scan, watcher incremental ingestion, Source Control visibility, repo docs/tree projection visibility, graph projection, and ledger export/dump. No code changes were required.

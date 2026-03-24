# Claude Opus Handoff

Date: 2026-03-24
Repo: `/home/develata/gitclone/Deve-Notebook`
Branch: `main`
Status: working tree clean, `HEAD` is 11 commits ahead of `origin/main`

## Project Snapshot

This is the Rust workspace for Deve-Notebook:

- `crates/core`: ledger, sync, source control, security, plugin runtime
- `apps/cli`: Axum server, WebSocket/session/scope control plane, admin and repo-scoped handlers
- `apps/web`: Leptos frontend

Current work has been concentrated in `apps/cli/src/server` and `crates/core/src/ledger/manager`, with a consistent theme:

- fail-closed repo scope resolution
- fail-closed listing/switcher/runtime binding cleanup
- remote catalog corruption classified as storage corruption, not plain absence
- local counterpart resolution kept UUID-first / URL-second
- active modules split down to reduce risk while finishing the repo-scope cleanup push

## Commits Ahead Of `origin/main`

Current ahead chain:

```text
3f728fd refactor: split repo scope control plane
1509533 fix: fail closed on broken remote catalogs and duplicate local urls
ee8ac18 fix: classify missing docs sources as not found
f1b6be0 refactor: split listing scope tests
87a2263 fix: fail closed plugin managed path symlink escapes
72eb1c2 fix: fail closed plugin managed path symlink escapes
b0e8dd3 refactor: split listing scope guards
bfc86be refactor: split listing repo and shadow handlers
325de8a refactor: split web source control message dispatch
c3347c7 refactor: split websocket broadcast filter helpers
daf8658 refactor: split switcher branch and repo handlers
```

Notes:

- `87a2263` and `72eb1c2` have the same subject line. Keep that in mind if you rebase or prepare a cleaned PR history.
- No uncommitted changes are present right now.

## What Was Just Finished

### 1. Remote catalog corruption now fails closed

Recent fixes made broken `remotes/` state surface as storage corruption instead of being disguised as a stale/missing branch.

Key files:

- `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/shadow_scope.rs`
- `/home/develata/gitclone/Deve-Notebook/crates/core/src/ledger/shadow/management.rs`
- `/home/develata/gitclone/Deve-Notebook/crates/core/src/ledger/manager/core_dirs.rs`
- `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_error.rs`
- `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/error_classify.rs`

Behavior now:

- missing `ledger/remotes/` root is reported as `Broken remote repo catalog`
- this maps to `StoragePersistFailed`
- it no longer gets silently downgraded to ordinary remote branch absence

### 2. Duplicate local URL ownership now fails closed

`find_local_repo_name_by_url(...)` used to quietly return `None` on duplicate URL owners. It now errors explicitly and forces callers to stop rather than guess.

Key files:

- `/home/develata/gitclone/Deve-Notebook/crates/core/src/ledger/manager/repo_lookup.rs`
- `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_recovery_test_local_counterpart.rs`

Behavior now:

- duplicate local URL owners produce an `Ambiguous local repository selector ...` error
- remote-to-local counterpart recovery does not pick an arbitrary repo

### 3. Repo-scope control plane was split without semantic changes

The latest commit is a pure control-plane split to make the remaining cleanup safer.

Key files:

- `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope.rs`
- `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_resolve.rs`
- `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_counterpart.rs`

Intent:

- keep `repo_scope.rs` thin
- isolate session resolution / stale-binding cleanup from counterpart mapping
- reduce risk while continuing the repo-scope / listing / switcher cleanup series

## Important Current Behavior

These invariants look intentional and should be preserved:

- repo-scoped flows are UUID-first; display names are selectors, not truth
- stale local selector or stale UUID pair must fail closed, not self-heal
- remote shadow branch availability is validated before treating branch scope as usable
- broken catalogs map to storage corruption, not repo-not-found
- repo-scoped messages must carry `repo_id`, `branch`, and `scope_nonce`
- single-repo local entrypoints may bootstrap local scope, but only in strictly local/unbound cases

## Validation Already Run

Recent successful checks:

```text
cargo fmt --all
cargo test -p deve_cli resolve_session_repo_preserves_missing_remote_catalog_failure -- --nocapture
cargo test -p deve_cli list_shadows_on_missing_remote_catalog_reports_storage_corruption -- --nocapture
cargo test -p deve_cli listing_shadow_scope_ -- --nocapture
cargo test -p deve_cli repo_scope_ -- --nocapture
cargo test -p deve_cli switch_ -- --nocapture
cargo check -p deve_core -p deve_cli -p deve_web
cargo clippy -p deve_core -p deve_cli -p deve_web --all-targets --all-features -- -D warnings
```

Everything above was green at the end of the session.

## Likely Remaining Work

The broad direction is still:

- repo-scope
- listing
- repair
- scoped runtime

What appears left is mostly edge cleanup, not large feature work.

### Highest-value next sweep

Look for remaining cases where broken state is still disguised as ordinary absence, especially:

- listing-related cleanup paths
- repair entrypoints
- scoped runtime/session cleanup
- source-control flows that still distinguish tests from production helpers

Suggested grep themes:

```text
Broken remote repo catalog
Broken local repo
StoragePersistFailed
ScRepoContextInvalid
SyncRepoUnbound
try_exists
metadata(
read_dir(
.ok().flatten()
unwrap_or_default()
warn!(
```

### Low-risk leftovers already seen

These were noted but not treated as production bugs:

- `/home/develata/gitclone/Deve-Notebook/apps/cli/src/commands/live_proxy.rs`
  `repo_query(...).unwrap_or_default()` is just building an optional query vector.
- `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/handlers/source_control/present.rs`
  `.ok().flatten()` is in `#[cfg(test)]` helper code; production handlers already use the strict path.

## Suggested Claude Opus Start Order

1. Read these files first:
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope.rs`
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_resolve.rs`
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_counterpart.rs`
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/shadow_scope.rs`
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/error_classify.rs`
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_error.rs`
2. Then inspect the focused tests:
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_test.rs`
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_recovery_test_local_counterpart.rs`
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/listing_shadow_scope_catalog_test.rs`
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/repo_scope_local_alias_test.rs`
   - `/home/develata/gitclone/Deve-Notebook/apps/cli/src/server/switcher_branch_scope_test.rs`
3. Continue scanning for “broken catalog or metadata silently treated as missing” patterns.
4. Prefer targeted tests over full-suite churn.

## Practical Notes For Handoff

- Current branch is clean. No stash required.
- The repo is in a good stop point for a new agent to continue immediately.
- The current direction is cleanup hardening, not feature expansion.
- If preparing a PR, consider squashing or cleaning the duplicate plugin-escape commits.

## Short Handoff Prompt

Use this if you want to drop context into Claude Opus quickly:

```text
You are taking over Deve-Notebook at /home/develata/gitclone/Deve-Notebook.
The repo is on main, clean, and 11 commits ahead of origin/main.
Recent work focused on fail-closed repo-scope/listing/runtime hardening, especially:
- broken remotes/ root now maps to Broken remote repo catalog -> StoragePersistFailed
- duplicate local URL owners now fail closed instead of returning None
- repo_scope control-plane was split into repo_scope.rs + repo_scope_resolve.rs + repo_scope_counterpart.rs without semantic change

Start by reading:
- apps/cli/src/server/repo_scope.rs
- apps/cli/src/server/repo_scope_resolve.rs
- apps/cli/src/server/repo_scope_counterpart.rs
- apps/cli/src/server/shadow_scope.rs
- apps/cli/src/server/error_classify.rs
- apps/cli/src/server/repo_scope_error.rs

Then inspect focused tests:
- apps/cli/src/server/repo_scope_test.rs
- apps/cli/src/server/repo_scope_recovery_test_local_counterpart.rs
- apps/cli/src/server/listing_shadow_scope_catalog_test.rs
- apps/cli/src/server/repo_scope_local_alias_test.rs
- apps/cli/src/server/switcher_branch_scope_test.rs

Continue the cleanup theme: look for any remaining path where broken catalog/metadata/runtime state is still disguised as ordinary absence instead of failing closed.
Prefer targeted cargo tests and preserve UUID-first / scope_nonce / repo-scoped invariants.
```

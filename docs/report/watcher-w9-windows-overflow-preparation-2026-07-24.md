# W9 Windows Overflow C2 Preparation

Status: `LOCAL_PREPARATION_COMPLETE / EXTERNAL_REVISION_REQUIRED`

Baseline: `main@fcabaa547f04e634926037bc8397124ba034b375`

Investigation started on 2026-07-23. This preparation report was refreshed and
completed on 2026-07-24.

This report is dated execution evidence. The live contract remains
`docs/plan/03_storage/watcher.md#watcher-contract`.

## Official-source review

- The newest stable `notify` release is still `8.2.0`. The newer `9.0.0`
  artifacts are release candidates, not a stable replacement:
  <https://crates.io/crates/notify>.
- The `notify` default branch still ignores the completion byte count and does
  not map `ERROR_NOTIFY_ENUM_DIR` to `Flag::Rescan` in
  `notify/src/windows.rs`:
  <https://github.com/notify-rs/notify/blob/main/notify/src/windows.rs>.
- Microsoft documents both required recovery signals: a successful
  `ReadDirectoryChangesW` completion with zero bytes means the change buffer was
  discarded, and `ERROR_NOTIFY_ENUM_DIR` requires the caller to enumerate the
  directory:
  <https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-readdirectorychangesw>
  and
  <https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--1000-1299->.
- Upstream issue `notify-rs/notify#963` tracks this exact gap:
  <https://github.com/notify-rs/notify/issues/963>.
- The open PR `notify-rs/notify#964` includes the required signals. Its
  2026-07-24 head also changes `notify/CHANGELOG.md` and emits an additional
  rearm error after the rescan event when rearming fails. Earlier revisions
  explored broader parser/error changes; the current head has narrowed, but it
  still is not the exact reviewed tag-based patch and its dependency revision
  remains external and unauthorized:
  <https://github.com/notify-rs/notify/pull/964>.

Conclusion: C1 is unavailable. W9 must continue through C2, but the existing
open PR is not accepted as the project dependency source because its behavior
surface exceeds the approved minimal patch.

## Prepared local artifacts

- Minimal upstream patch:
  `tools/patches/notify/8.2.0-windows-overflow-rescan.patch`.
- Base tag: `notify-8.2.0`.
- Base revision: `a1d7c2d8f80786679d58ec6d5986a1d4278bc8cf`.
- Allowed changed production file: `notify/src/windows.rs`.
- Behavior:
  - zero-byte successful completion requests `Flag::Rescan`;
  - `ERROR_NOTIFY_ENUM_DIR` requests `Flag::Rescan`;
  - the next `ReadDirectoryChangesW` request is armed before the rescan event is
    published;
  - public API, buffer size, non-Windows code and other Windows error branches
    remain unchanged.
- Upstream unit tests cover both overflow signals, the normal non-empty success
  branch and the public rescan flag.
- Project producer preparation lives in
  `crates/core/tests/watcher_windows_overflow.rs` and
  `crates/core/tests/watcher_windows_overflow_support/mod.rs`. Both entrypoints are
  ignored by ordinary test runs. The parent applies a 120-second hard timeout
  to each of exactly three child processes, requires distinct process IDs and
  identical host/convergence claims, and publishes child results and the final
  claims file through same-directory no-clobber atomic persistence. Child
  stdout/stderr are drained to files instead of bounded pipes; timeout and poll
  failures request termination, make a bounded reap attempt and preserve
  cleanup diagnostics without waiting indefinitely.

The patch was formatted and its four focused unit tests passed on Windows with
Rust/Cargo `1.97.0`. The upstream repository currently resolves a transitive
`getrandom 0.4.3` manifest that Cargo `1.77.2` cannot parse because it uses the
Edition 2024 manifest feature; this is an upstream unlocked-dependency/MSRV
reproduction issue, not a compile failure in the patch.

The split project producer was formatted and compiled with:

```text
cargo fmt --all -- --check
cargo test --locked -p deve_core --test watcher_windows_overflow --no-run
```

The ordinary integration-test entrypoint also runs local dependency-binding
fixtures while leaving both real overflow entrypoints ignored. The fixtures
accept the two explicitly prepared `notify-types` identity shapes and reject a
second `notify`/`notify-types` identity, alternate registry, unapproved version,
git source/revision mismatch and missing notify registry entry.

## Real Windows preparation probe

A single child process was run from a temporary detached worktree against the
reviewed local path patch. It produced:

- Windows: `Microsoft Windows [Version 10.0.26200.8875]`;
- filesystem: `NTFS`;
- burst files: `2048`;
- Rescan observed: `true`;
- normal event after rearm observed: `true`;
- reconciled pending entries: `2050`;
- expected SHA-256:
  `1417922ff730013ad418d23e58656028e0d69790c17da255a0dcb104efd8b2be`;
- actual SHA-256:
  `1417922ff730013ad418d23e58656028e0d69790c17da255a0dcb104efd8b2be`.

This is preparation evidence only. It used a local path override, ran only one
child process, was not bound to an authorized immutable dependency revision,
and did not run through `acceptance-run` on a clean exact HEAD. It therefore
does not replace the `STORE-016` gap. It also predates the final split producer
and barrier-acknowledgement/process-cleanup revisions, so it is not execution
evidence for the final two-file producer. The final producer is compile-checked
and its dependency-binding fixtures are executed locally; the real overflow
entrypoints remain intentionally unexecuted until an authorized immutable
source exists.

## Dependency identity constraint

Overriding only the `notify` package with the reviewed git/path source does not
compile against the current product dependency graph:

- the upstream workspace binds `notify` to its path-based `notify-types 2.0.0`;
- `notify-debouncer-full 0.7.0` and `notify-debouncer-mini 0.7.0` resolve the
  crates.io `notify-types 2.1.0`;
- Cargo treats these as distinct package identities, so their `Event` and
  `EventKind` types are incompatible.

A temporary experiment showed that overriding both `notify` and
`notify-types` from the same source tree, with `notify-types 2.0.0`, restores a
single package identity and lets the one-process Windows probe pass. That
experiment also changes the product from registry `notify-types 2.1.0` to
source-bound `2.0.0`, so it is not authorized for `Cargo.toml` or `Cargo.lock`.

The prepared producer rejects multiple `notify-types` identities. It can
validate either eventual safe source shape:

1. git-bound `notify` plus one registry `notify-types` identity, if an
   authorized branch can use the existing crates.io `notify-types 2.1.0`
   identity without widening behavior;
2. git-bound `notify` and `notify-types` from the same source and exact revision,
   with source-bound `notify-types 2.0.0` and both entries explicitly recorded
   in the dependency override registry.

The source shape remains an explicit post-commit USER decision.

## Dependency cut remains intentionally absent

This preparation does **not** add any of the following:

- a `[patch.crates-io]` entry;
- a Cargo.lock source rewrite;
- `docs/registry/dependency-source-overrides.json`;
- a replacement for `gap.watcher.windows-overflow-receipt`;
- a `watcher_runtime = 已收敛` claim.

Those changes are forbidden until an authorized fork or upstream branch
provides a resolvable immutable 40-character revision containing the reviewed
patch.

## Proposed upstream PR text

Title:

```text
fix(windows): emit rescan after ReadDirectoryChangesW overflow
```

Body:

```text
ReadDirectoryChangesW reports an unusable notification buffer in two ways:

- ERROR_SUCCESS with zero bytes transferred;
- ERROR_NOTIFY_ENUM_DIR.

Treat both completions as a rescan request. The watcher is rearmed before the
Flag::Rescan event is emitted, so callers can enumerate current state and the
same watcher continues to receive later changes.

This patch intentionally does not change the buffer size, public API, record
parser, other Windows error handling, or non-Windows backends.

Tests cover both overflow signals, a normal non-empty completion, and the
public rescan flag.
```

## External authorization boundary

The next step changes external state and therefore requires separate USER
authorization:

1. choose whether the immutable source will keep one registry
   `notify-types` identity or bind both packages to one source revision;
2. create or select a fork branch based on
   `a1d7c2d8f80786679d58ec6d5986a1d4278bc8cf`;
3. apply only the prepared patch and push that branch;
4. optionally submit the PR text above upstream;
5. return the resulting immutable 40-character revision to Deve-Notebook;
6. only then add the approved git override, lockfile binding,
   dependency-source registry row(s) and execute the three-process Windows
   receipt producer.

Until step 5 succeeds on one exact Deve-Notebook HEAD, `STORE-016`, W9, W10 and
tag-ready remain blocked.

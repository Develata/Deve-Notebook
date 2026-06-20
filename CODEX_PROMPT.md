# Prompt for codex (paste into `codex exec` on branch codex/native-peer-modes)

You are picking up branch `codex/native-peer-modes`. Three new commits sit on
top of your work (`21c70e2b` and earlier). Read `HANDOFF.md` at repo root first
for full context; summary below.

## What landed (do NOT redo)

1. `6e480b13 Version ledger and redb storage formats` — your storage versioning
   (#A ledger `DEVELDG1` envelope, #C redb `REPO_SCHEMA_VERSION` gate, #B
   lockstep WS protocol) was recovered via non-destructive cherry-pick after a
   detached-HEAD mishap. It is intact on the branch. No action needed.

2. Three commits fixing all 16 pre-existing test failures:
   - `50f18059` (test only): projection/edit fixtures now call
     `ensure_local_repo_workspace_identity` in setup, mirroring production init,
     so they stop tripping the `72a3f069` identity gate. 14 tests.
   - `fa586571` (test only): writer-registration cleanup test now uses a browser
     session, matching the `cf7ad475` browser-only writer gate. 1 test.
   - `6069541c` (PRODUCTION): `take_staged_for_target` (unstage's only caller)
     now prefers an exact staged-path match for doc-scoped targets; stage/read
     paths and `select_entry_for_doc` keep "live successor wins". This reconciles
     two contradictory tests (stage vs unstage resolution differ by design) and
     removes a non-atomic half-migration risk when unstaging the deleted side of
     a rename pair. 1 test.

## Verified (fresh this session)

- `cargo test -p deve_core --lib` → 587 passed / 0 failed
- `cargo test -p deve_cli --lib source_control` → 169 passed / 0 failed
- All 11 source_control/discard integration files green; all 16 original
  failing surfaces green.

## Your tasks

1. **Review `6069541c`** (the only production change). Confirm the unstage
   exact-path-wins / stage live-successor-wins split matches the intended
   source-control target-resolution contract. The split is intentional — do not
   collapse it back to a single shared resolver. If you disagree, say why before
   changing.

2. **Continue the `tools/baseline` independence work** per your own plan
   (extract `deve_baseline` to `tools/baseline`, migrate `check-release-baseline.sh`
   text/order/git checks). The three new commits do not touch `tools/` or the
   baseline crate, so there is no conflict.

3. **(Optional) Pin rustfmt version.** `crates/core/tests/common/mod.rs` and
   `staging/target/tests.rs` keep producing cosmetic import-reorder / `assert!`
   reformat diffs from a local rustfmt version mismatch. Consider fixing the
   toolchain rustfmt version so this noise stops.

## Coordination

Avoid concurrent git reset/checkout on the main worktree while the other agent
is editing it — this session hit a detached-HEAD "lost commit" scare from
exactly that. Use a separate `git worktree` for read-only diagnosis during
concurrent work.

Do NOT amend or rewrite the three landed commits; build on top.

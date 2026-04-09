# Architecture Diff Report (doc vs code)

Generated: 2026-04-09 (manual baseline)

This report compares [`architecture-doc.lisp`](./architecture-doc.lisp) (derived from `docs/plan/`, `docs/features/`, `docs/acceptance-cases/`) against [`architecture-code.lisp`](./architecture-code.lisp) (derived from the actual source tree). Nodes marked with `*` in the SVG diagram correspond to the divergences listed here.

## Summary

| Metric | Doc view | Code view | Divergence |
|---|---|---|---|
| CLI commands | 8 | 12 | **+4 in code** |
| HTTP routes (sc/repo/auth/admin) | aggregated | 18 concrete routes | code is finer-grained |
| Core subsystems | 9 (incl. watcher root) | 9 + `core-misc` catch-all | +1 in code |
| Module-layer nodes | 18 | 26 | **+8 in code** (mostly core-misc) |

---

## 1. CLI commands — **4 commands missing from docs** `*`

**Code has but doc does not mention in `12_commands.md §1. CLI Commands`:**

| Command | Code file | Suggested plan chapter |
|---|---|---|
| `deve node-check` | `apps/cli/src/commands/node_check.rs` | `04_storage` (repair path) or `12_commands` |
| `deve recover` | `apps/cli/src/commands/recover.rs` | `04_storage #9 Recovery / Repair` |
| `deve repair` | `apps/cli/src/commands/repair.rs` | `04_storage #9 Recovery / Repair` |
| `deve live-proxy` | `apps/cli/src/commands/live_proxy.rs` | new: `05_network` live-proxy mode |

**Recommended fix**: Extend `docs/plan/12_commands.md §1. CLI Commands` with these 4 commands, or (if any are internal-only) add a `### 1.1 Internal Debug Commands` sub-section and document their purpose.

---

## 2. HTTP routes — code is more granular than docs describe

**Code has concrete routes** (from `apps/cli/src/server/router.rs`):

```
/ws
/api/sc/{pending, status, diff, commits, commit-diff,
         stage-pending, unstage, discard-pending, commit}
/api/repo/{docs, doc}
/api/auth/{login, logout, me}
/api/admin/{dump, export, node-check}
/api/node/role
```

**Doc describes them abstractly** in `07_diff_logic.md`, `06_repository.md`, `09_auth.md` without listing endpoints.

**Divergence level**: Low — abstraction difference, not a structural conflict. Acceptable as long as the `Counterpart Feature` / `Counterpart Acceptance` fields in each plan Metadata point to the same behavior.

**Recommended fix**: Add a `## N. HTTP Endpoints` table to `05_network.md` or `09_auth.md` enumerating each route and its handler location. This becomes the doc-side anchor for `architecture-doc.lisp`.

---

## 3. Core subsystems — `core-misc` catch-all in code

**Code has** (under `crates/core/src/`):

```
ledger/ sync/ source_control/ tree/ protocol/ security/
plugin/ search/ mcp/ skill/ context/ utils/
+ top-level files: watcher.rs vfs.rs state.rs models.rs config.rs error.rs
```

**Doc only covers** the first 8 + watcher as root. The following exist in code but have no dedicated plan chapter:

| Module | Code path | Doc status |
|---|---|---|
| `core::context` | `crates/core/src/context/` | Not described — used by runtime state assembly |
| `core::mcp` | `crates/core/src/mcp/` | Not described — MCP integration |
| `core::skill` | `crates/core/src/skill/` | Not described — skill registry |
| `core::utils` | `crates/core/src/utils/` | Not described — infra helpers |
| `core::vfs` | `crates/core/src/vfs.rs` | Not described — virtual fs layer |
| `core::state` | `crates/core/src/state.rs` | Not described — app state assembly |
| `core::models` | `crates/core/src/models.rs` | Partially in `01_terminology` |
| `core::config` | `crates/core/src/config.rs` | Partially in `13_settings` |
| `core::error` | `crates/core/src/error.rs` | Not described |

**Recommended fix**:
- `context`, `mcp`, `skill` should be mentioned in `10_ai_agent.md §2. Native AI Chat` or `17_plugins.md` — they are the infrastructure for AI/plugin runtime.
- `vfs`, `state`, `models`, `config`, `error`, `utils` are infrastructure glue — acceptable to document as "infra layer" in `01_terminology.md` or a new `AGENTS.md` section.

---

## 4. Module layer — watcher/drift_detect are well-covered

**Matching (no `*`)**:
- `sync::watcher` ↔ `04_storage#watcher-contract` ✓
- `sync::drift_detect` ↔ `04_storage#projection-contract` ✓
- `source_control::pending_fs` ↔ `04_storage#watcher-contract` ✓
- `apps/cli export commands` ↔ `04_storage#backup-export` ✓
- `server/auth/headers.rs` ↔ `09_auth#security-headers` ✓

These are the 5 anchors already enforced by `plan_ref` bijection. No drift.

---

## 5. Next actions to reach zero divergence

1. **Document the 4 missing CLI commands** in `docs/plan/12_commands.md §1` → updates `architecture-doc.lisp`.
2. **Enumerate HTTP routes** in `docs/plan/05_network.md` or `09_auth.md` → doc view becomes as granular as code view.
3. **Classify core-misc modules** in `docs/plan/` → assign each to an existing chapter or add an infra chapter.
4. **Regenerate** `architecture-doc.lisp` and `architecture-code.lisp`, run diff again, verify zero `*` markers.

Once all 4 are addressed, the `*` markers in `architecture.dot` can be removed and `architecture.svg` becomes a faithful blueprint.

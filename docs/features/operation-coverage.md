# Operation Coverage Registry

This registry binds operation files to acceptance-case identifiers. It is a manual coverage index for the operation-level architecture slice; keep it aligned with `docs/features/operations/*.md` and `docs/overview/architecture-diff.md`.

`docs/features/operations/00_schema.md` defines the schema and is not itself a flow.

| Flow ID | Operation File | Acceptance Cases |
|---|---|---|
| `flow.ai.chat` | [`ai_chat.md`](./operations/ai_chat.md) | `AI-FEAT-01` |
| `flow.ai.trusted-external-agent-boundary` | [`trusted_external_agent_boundary.md`](./operations/trusted_external_agent_boundary.md) | `AI-005`, `AI-006`, `PLUG-001` |
| `flow.auth.login` | [`auth_login.md`](./operations/auth_login.md) | `AUTH-001`, `AUTH-006`, `AUTH-011` |
| `flow.auth.session-unauthorized` | [`auth_session_unauthorized.md`](./operations/auth_session_unauthorized.md) | `AUTH-002`, `AUTH-003`, `AUTH-011` |
| `flow.cli.control-commands` | [`cli_control_commands.md`](./operations/cli_control_commands.md) | `CMD-001`, `CMD-002`, `CMD-003`, `CMD-004` |
| `flow.doc.edit-confirmed-op` | [`doc_edit_confirmed_op.md`](./operations/doc_edit_confirmed_op.md) | `RENDER-FEAT-01`, `STORAGE-FEAT-01`, `STORAGE-FEAT-02` |
| `flow.doc.pending-navigation-guard` | [`doc_pending_navigation_guard.md`](./operations/doc_pending_navigation_guard.md) | `WEBWRITE-FEAT-01`, `WEBWRITE-FEAT-02`, `WEBWRITE-FEAT-03` |
| `flow.i18n.locale-error` | [`i18n_locale_error.md`](./operations/i18n_locale_error.md) | `I18N-001`, `I18N-002`, `I18N-003`, `I18N-004`, `I18N-005`, `I18N-006` |
| `flow.net.key-exchange` | [`net_key_exchange.md`](./operations/net_key_exchange.md) | `NET-FEAT-01`, `NET-FEAT-03` |
| `flow.net.sync-handshake` | [`net_sync_handshake.md`](./operations/net_sync_handshake.md) | `NET-FEAT-01`, `NET-FEAT-02`, `NET-FEAT-03` |
| `flow.net.sync-transfer` | [`net_sync_transfer.md`](./operations/net_sync_transfer.md) | `NET-FEAT-02`, `NET-FEAT-03` |
| `flow.plugin.runtime-boundary` | [`plugin_runtime_boundary.md`](./operations/plugin_runtime_boundary.md) | `PLUG-001`, `AI-005`, `AI-006` |
| `flow.release.ci` | [`release_ci.md`](./operations/release_ci.md) | `REL-001`, `REL-002`, `REL-003`, `TECH-001`, `PERF-001` |
| `flow.rendering.cursor-reveal` | [`rendering_cursor_reveal.md`](./operations/rendering_cursor_reveal.md) | `RENDER-CURSOR-001`, `RENDER-RICH-002` |
| `flow.rendering.math-mermaid` | [`rendering_math_mermaid.md`](./operations/rendering_math_mermaid.md) | `RENDER-MATH-001`, `RENDER-MERMAID-001`, `RENDER-BLOCK-001` |
| `flow.repo.branch-switch` | [`repo_branch_switch.md`](./operations/repo_branch_switch.md) | `CMD-004`, `REPO-FEAT-02`, `REPO-FEAT-03` |
| `flow.repo.file-operations` | [`repo_file_operations.md`](./operations/repo_file_operations.md) | `REPO-FEAT-01`, `UI-DESK-003` |
| `flow.repo.open-doc` | [`repo_open_doc.md`](./operations/repo_open_doc.md) | `CMD-003`, `REPO-FEAT-01`, `STORE-009` |
| `flow.repo.switch` | [`repo_switch.md`](./operations/repo_switch.md) | `REPO-FEAT-01`, `REPO-FEAT-03` |
| `flow.sc.commit` | [`sc_commit.md`](./operations/sc_commit.md) | `DIFF-FEAT-01`, `DIFF-FEAT-03` |
| `flow.sc.commit-and-push` | [`sc_commit_and_push.md`](./operations/sc_commit_and_push.md) | `DIFF-FEAT-02` |
| `flow.sc.discard-file` | [`sc_discard_file.md`](./operations/sc_discard_file.md) | `DIFF-FEAT-01`, `DIFF-FEAT-03` |
| `flow.sc.discard-pending` | [`sc_discard_pending.md`](./operations/sc_discard_pending.md) | `DIFF-FEAT-03` |
| `flow.sc.history-commit-diff` | [`sc_history_commit_diff.md`](./operations/sc_history_commit_diff.md) | `DIFF-FEAT-02` |
| `flow.sc.merge-peer` | [`sc_merge_peer.md`](./operations/sc_merge_peer.md) | `DIFF-002`, `DIFF-003`, `DIFF-005` |
| `flow.sc.merge-runtime` | [`sc_merge_runtime.md`](./operations/sc_merge_runtime.md) | `DIFF-005`, `NET-FEAT-03` |
| `flow.sc.resolve-conflict` | [`sc_resolve_conflict.md`](./operations/sc_resolve_conflict.md) | `DIFF-FEAT-03` |
| `flow.sc.stage-unstage` | [`sc_stage_unstage.md`](./operations/sc_stage_unstage.md) | `DIFF-FEAT-01`, `DIFF-FEAT-03` |
| `flow.search.query` | [`search_query.md`](./operations/search_query.md) | `UI-DESK-003`, `UI-MOB-007` |
| `flow.settings.update` | [`settings_update.md`](./operations/settings_update.md) | `SET-001`, `SET-002`, `CMD-001` |
| `flow.tech-stack.runtime-budget` | [`tech_stack_runtime_budget.md`](./operations/tech_stack_runtime_budget.md) | `TECH-001`, `PERF-001`, `REL-003` |
| `flow.ui.command-palette` | [`ui_command_palette.md`](./operations/ui_command_palette.md) | `UI-GEN-002`, `UI-GEN-003`, `CMD-002` |

## Maintenance Rules

1. Add a row here when adding an operation file.
2. Keep `Acceptance Cases` equal to the operation file metadata line.
3. Keep architecture flow labels in `docs/overview/architecture-diff.md`; this file tracks flow IDs and acceptance coverage.

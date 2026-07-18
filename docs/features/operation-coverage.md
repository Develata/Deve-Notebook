<!-- This file is a projection of docs/plan/20_operations_catalog.md -->
<!-- It binds flow IDs to acceptance cases; it MUST NOT define op semantics or introduce flow IDs absent from chapter 20 (the authoritative operation catalog). -->

# Operation Coverage Registry

This registry binds operation files to acceptance-case identifiers. It is a manual coverage index for the operation-level architecture slice; keep it aligned with `docs/features/operations/*.md` and `docs/overview/architecture-diff.md`.

The authoritative operation catalog is `docs/plan/20_operations_catalog.md`; this file is its acceptance-binding projection. The `Flow ID` set here and chapter 20's operation-flow catalog MUST stay strictly 1:1.

`docs/features/operations/00_schema.md` defines the schema and is not itself a flow.

| Flow ID | Operation File | Acceptance Cases |
|---|---|---|
| `flow.ai.chat` | [`ai_chat.md`](./operations/ai_chat.md) | `AI-FEAT-01`, `AI-002`, `AI-003`, `AI-008`, `AI-009` |
| `flow.ai.trusted-external-agent-boundary` | [`trusted_external_agent_boundary.md`](./operations/trusted_external_agent_boundary.md) | `AI-005`, `AI-006`, `PLUG-001` |
| `flow.auth.login` | [`auth_login.md`](./operations/auth_login.md) | `AUTH-001`, `AUTH-006`, `AUTH-011` |
| `flow.auth.session-unauthorized` | [`auth_session_unauthorized.md`](./operations/auth_session_unauthorized.md) | `AUTH-002`, `AUTH-003`, `AUTH-011`, `AUTH-012` |
| `flow.commands.surface-action-routing` | [`command_surface_action_routing.md`](./operations/command_surface_action_routing.md) | `CMD-002`, `CMD-003`, `CMD-004`, `REPO-FEAT-01`, `REPO-FEAT-02` |
| `flow.commands.surface-mode-routing` | [`command_surface_mode_routing.md`](./operations/command_surface_mode_routing.md) | `CMD-002`, `CMD-003`, `CMD-004`, `UI-DESK-003` |
| `flow.cli.control-commands` | [`cli_control_commands.md`](./operations/cli_control_commands.md) | `CMD-001`, `CMD-002`, `CMD-003`, `CMD-004` |
| `flow.cli.empty-command-guidance` | [`cli_empty_command_guidance.md`](./operations/cli_empty_command_guidance.md) | `CMD-001` |
| `flow.cli.export-inspect` | [`cli_export_inspect.md`](./operations/cli_export_inspect.md) | `CMD-001`, `CMD-008`, `TECH-002` |
| `flow.cli.help-surface` | [`cli_help_surface.md`](./operations/cli_help_surface.md) | `CMD-001`, `CMD-006`, `CMD-007`, `CMD-009` |
| `flow.cli.parse-command` | [`cli_parse_command.md`](./operations/cli_parse_command.md) | `CMD-001`, `CMD-006`, `CMD-007`, `CMD-008`, `CMD-009` |
| `flow.cli.repair-admin` | [`cli_repair_admin.md`](./operations/cli_repair_admin.md) | `CMD-001`, `CMD-009`, `REPO-FEAT-03` |
| `flow.cli.runtime-handoff` | [`cli_runtime_handoff.md`](./operations/cli_runtime_handoff.md) | `CMD-001`, `CMD-006`, `CMD-007`, `CMD-008`, `CMD-009` |
| `flow.cli.server-runtime` | [`cli_server_runtime.md`](./operations/cli_server_runtime.md) | `CMD-001`, `CMD-007`, `REL-002` |
| `flow.cli.projection-workspace-indexing` | [`cli_projection_workspace_indexing.md`](./operations/cli_projection_workspace_indexing.md) | `CMD-001`, `CMD-006` |
| `flow.doc.edit-confirmed-op` | [`doc_edit_confirmed_op.md`](./operations/doc_edit_confirmed_op.md) | `RENDER-FEAT-01`, `STORAGE-FEAT-01`, `STORAGE-FEAT-02`, `WEBWRITE-FEAT-01`, `WEBWRITE-FEAT-02`, `WEBWRITE-FEAT-03` |
| `flow.doc.pending-navigation-guard` | [`doc_pending_navigation_guard.md`](./operations/doc_pending_navigation_guard.md) | `WEBNAV-FEAT-01`, `WEBNAV-FEAT-02`, `WEBNAV-FEAT-03` |
| `flow.external-changes` | [`external_changes.md`](./operations/external_changes.md) | `DIFF-011`, `UI-MOB-024` |
| `flow.i18n.error-mapping` | [`i18n_error_mapping.md`](./operations/i18n_error_mapping.md) | `I18N-004`, `I18N-006`, `AUTH-002` |
| `flow.i18n.hardcoded-audit` | [`i18n_hardcoded_audit.md`](./operations/i18n_hardcoded_audit.md) | `I18N-001`, `I18N-003` |
| `flow.i18n.locale-error` | [`i18n_locale_error.md`](./operations/i18n_locale_error.md) | `I18N-001`, `I18N-002`, `I18N-003`, `I18N-004`, `I18N-005`, `I18N-006` |
| `flow.i18n.locale-surface-switch` | [`locale_surface_switch.md`](./operations/locale_surface_switch.md) | `I18N-002`, `SET-005`, `CMD-002` |
| `flow.i18n.locale-selection` | [`i18n_locale_selection.md`](./operations/i18n_locale_selection.md) | `I18N-002`, `I18N-003`, `SET-005` |
| `flow.i18n.localized-formatting` | [`i18n_localized_formatting.md`](./operations/i18n_localized_formatting.md) | `I18N-005`, `TECH-001` |
| `flow.net.key-exchange` | [`net_key_exchange.md`](./operations/net_key_exchange.md) | `NET-FEAT-01`, `NET-FEAT-03` |
| `flow.net.sync-handshake` | [`net_sync_handshake.md`](./operations/net_sync_handshake.md) | `NET-FEAT-01`, `NET-FEAT-02`, `NET-FEAT-03` |
| `flow.net.sync-transfer` | [`net_sync_transfer.md`](./operations/net_sync_transfer.md) | `NET-FEAT-02`, `NET-FEAT-03`, `NET-FEAT-05` |
| `flow.plugin.runtime-boundary` | [`plugin_runtime_boundary.md`](./operations/plugin_runtime_boundary.md) | `PLUG-001`, `AI-005`, `AI-006` |
| `flow.remote-import.apply` | [`remote_import_apply.md`](./operations/remote_import_apply.md) | `STORE-021` |
| `flow.remote-import.manage` | [`remote_import_manage.md`](./operations/remote_import_manage.md) | `STORE-023` |
| `flow.remote-import.prepare` | [`remote_import_prepare.md`](./operations/remote_import_prepare.md) | `STORE-019` |
| `flow.remote-import.review` | [`remote_import_review.md`](./operations/remote_import_review.md) | `STORE-020` |
| `flow.remote-projection.push` | [`remote_projection_push.md`](./operations/remote_projection_push.md) | `STORE-018` |
| `flow.release.ci` | [`release_ci.md`](./operations/release_ci.md) | `REL-001`, `REL-002`, `REL-003`, `TECH-001`, `PERF-001` |
| `flow.release.tag-dispatch` | [`release_tag_dispatch.md`](./operations/release_tag_dispatch.md) | `REL-001`, `REL-003` |
| `flow.release.quality-gates` | [`release_quality_gates.md`](./operations/release_quality_gates.md) | `REL-003`, `REL-003A`, `REL-007`, `REL-008`, `TECH-001`, `PERF-001`, `REL-013`, `REL-014` |
| `flow.release.artifact-publish` | [`release_artifact_publish.md`](./operations/release_artifact_publish.md) | `REL-001`, `REL-002`, `REL-003` |
| `flow.release.delivery-verification` | [`release_delivery_verification.md`](./operations/release_delivery_verification.md) | `REL-001`, `REL-002`, `REL-003` |
| `flow.rendering.cursor-reveal` | [`rendering_cursor_reveal.md`](./operations/rendering_cursor_reveal.md) | `RENDER-CURSOR-001`, `RENDER-RICH-002` |
| `flow.rendering.checkbox-writeback` | [`rendering_checkbox_writeback.md`](./operations/rendering_checkbox_writeback.md) | `RENDER-RICH-001` |
| `flow.rendering.inline-source-reveal` | [`rendering_inline_source_reveal.md`](./operations/rendering_inline_source_reveal.md) | `RENDER-CURSOR-001`, `RENDER-RICH-002` |
| `flow.rendering.link-activation-gate` | [`rendering_link_activation_gate.md`](./operations/rendering_link_activation_gate.md) | `RENDER-LINK-001` |
| `flow.rendering.large-doc-prefetch` | [`rendering_large_doc_prefetch.md`](./operations/rendering_large_doc_prefetch.md) | `RENDER-LARGE-001`, `RENDER-LARGE-002` |
| `flow.rendering.large-doc-search-gate` | [`rendering_large_doc_search_gate.md`](./operations/rendering_large_doc_search_gate.md) | `RENDER-LARGE-001`, `UI-DESK-003` |
| `flow.rendering.math-mermaid` | [`rendering_math_mermaid.md`](./operations/rendering_math_mermaid.md) | `RENDER-MATH-001`, `RENDER-MERMAID-001`, `RENDER-BLOCK-001` |
| `flow.rendering.math-source-projection` | [`rendering_math_source_projection.md`](./operations/rendering_math_source_projection.md) | `RENDER-MATH-001`, `RENDER-BLOCK-001` |
| `flow.rendering.mermaid-source-projection` | [`rendering_mermaid_source_projection.md`](./operations/rendering_mermaid_source_projection.md) | `RENDER-MERMAID-001`, `RENDER-BLOCK-001` |
| `flow.rendering.outline-navigation` | [`rendering_outline_navigation.md`](./operations/rendering_outline_navigation.md) | `RENDER-OUTLINE-001` |
| `flow.rendering.projection-refresh` | [`rendering_projection_refresh.md`](./operations/rendering_projection_refresh.md) | `RENDER-BLOCK-001`, `RENDER-INLINE-001`, `RENDER-CURSOR-001` |
| `flow.repo.branch-switch` | [`repo_branch_switch.md`](./operations/repo_branch_switch.md) | `CMD-004`, `REPO-FEAT-02`, `REPO-FEAT-03` |
| `flow.repo.file-op-shell-routing` | [`repo_file_op_shell_routing.md`](./operations/repo_file_op_shell_routing.md) | `REPO-FEAT-01`, `UI-DESK-003` |
| `flow.repo.file-operations` | [`repo_file_operations.md`](./operations/repo_file_operations.md) | `REPO-FEAT-01`, `UI-DESK-003`, `STORE-012`, `STORE-013` |
| `flow.repo.alias-set` | [`repo_alias_set.md`](./operations/repo_alias_set.md) | `STORE-014A` |
| `flow.repo.alias-transfer` | [`repo_alias_transfer.md`](./operations/repo_alias_transfer.md) | `CMD-008` |
| `flow.repo.lifecycle` | [`repo_lifecycle.md`](./operations/repo_lifecycle.md) | `REPO-FEAT-03` |
| `flow.repo.open-doc` | [`repo_open_doc.md`](./operations/repo_open_doc.md) | `CMD-003`, `REPO-FEAT-01`, `STORE-009` |
| `flow.repo.switch` | [`repo_switch.md`](./operations/repo_switch.md) | `REPO-FEAT-01` |
| `flow.sc.commit` | [`sc_commit.md`](./operations/sc_commit.md) | `DIFF-FEAT-01`, `DIFF-FEAT-03`, `DIFF-009`, `AI-007` |
| `flow.sc.commit-and-push` | [`sc_commit_and_push.md`](./operations/sc_commit_and_push.md) | `DIFF-FEAT-02` |
| `flow.sc.discard-file` | [`sc_discard_file.md`](./operations/sc_discard_file.md) | `DIFF-FEAT-01`, `DIFF-FEAT-03` |
| `flow.sc.discard-pending` | [`sc_discard_pending.md`](./operations/sc_discard_pending.md) | `DIFF-FEAT-03` |
| `flow.sc.history-commit-diff` | [`sc_history_commit_diff.md`](./operations/sc_history_commit_diff.md) | `DIFF-FEAT-02` |
| `flow.sc.merge-peer` | [`sc_merge_peer.md`](./operations/sc_merge_peer.md) | `DIFF-002`, `DIFF-003`, `DIFF-004`, `DIFF-005` |
| `flow.sc.merge-runtime` | [`sc_merge_runtime.md`](./operations/sc_merge_runtime.md) | `DIFF-005`, `NET-FEAT-03` |
| `flow.sc.resolve-conflict` | [`sc_resolve_conflict.md`](./operations/sc_resolve_conflict.md) | `DIFF-FEAT-03` |
| `flow.sc.stage-unstage` | [`sc_stage_unstage.md`](./operations/sc_stage_unstage.md) | `DIFF-FEAT-01`, `DIFF-FEAT-03` |
| `flow.search.query` | [`search_query.md`](./operations/search_query.md) | `SEARCH-001`, `SEARCH-002`, `SEARCH-003`, `UI-DESK-003`, `UI-MOB-007` |
| `flow.settings.env-defaults` | [`settings_env_defaults.md`](./operations/settings_env_defaults.md) | `SET-001` |
| `flow.settings.feedback-render` | [`settings_feedback_render.md`](./operations/settings_feedback_render.md) | `SET-005`, `SET-006` |
| `flow.settings.file-config` | [`settings_file_config.md`](./operations/settings_file_config.md) | `SET-002`, `SET-004` |
| `flow.settings.persistence-apply` | [`settings_persistence_apply.md`](./operations/settings_persistence_apply.md) | `SET-001`, `SET-002`, `SET-003`, `SET-004` |
| `flow.settings.surface-open` | [`settings_surface_open.md`](./operations/settings_surface_open.md) | `SET-005`, `CMD-002` |
| `flow.settings.runtime-feedback` | [`settings_runtime_feedback.md`](./operations/settings_runtime_feedback.md) | `SET-003`, `SET-006`, `CMD-002`, `AI-006` |
| `flow.settings.ui-preferences` | [`settings_ui_preferences.md`](./operations/settings_ui_preferences.md) | `SET-005`, `I18N-001`, `I18N-002` |
| `flow.settings.update` | [`settings_update.md`](./operations/settings_update.md) | `SET-001`, `SET-002`, `CMD-001` |
| `flow.settings.value-mutation` | [`settings_value_mutation.md`](./operations/settings_value_mutation.md) | `SET-005`, `SET-002`, `CMD-001` |
| `flow.tech-stack.dependency-policy` | [`tech_stack_dependency_policy.md`](./operations/tech_stack_dependency_policy.md) | `TECH-001`, `PERF-001` |
| `flow.tech-stack.platform-release-channel` | [`tech_stack_platform_release_channel.md`](./operations/tech_stack_platform_release_channel.md) | `REL-001`, `REL-002`, `REL-003`, `TECH-001` |
| `flow.tech-stack.runtime-budget` | [`tech_stack_runtime_budget.md`](./operations/tech_stack_runtime_budget.md) | `TECH-001`, `PERF-001`, `REL-003` |
| `flow.tech-stack.runtime-budget-check` | [`tech_stack_runtime_budget_check.md`](./operations/tech_stack_runtime_budget_check.md) | `PERF-001`, `REL-003`, `TECH-001` |
| `flow.ui.command-palette` | [`ui_command_palette.md`](./operations/ui_command_palette.md) | `UI-GEN-002`, `UI-GEN-003`, `CMD-002` |
| `flow.ui.context-action-routing` | [`ui_context_action_routing.md`](./operations/ui_context_action_routing.md) | `UI-WEB-007` |

## Maintenance Rules

1. Register a new operation-flow (`Flow ID`) in `docs/plan/20_operations_catalog.md` first; only then add its row here. Never introduce a `Flow ID` that is absent from chapter 20.
2. Add a row here when adding an operation file.
3. Keep `Acceptance Cases` equal to the operation file metadata line.
4. Keep architecture flow labels in `docs/overview/architecture-diff.md`; this file tracks flow IDs and acceptance coverage.

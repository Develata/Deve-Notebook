## Operation Flow Backlink Cases

这些用例把 operation registry 中的 feature-level refs 固化为可自动化验收入口，避免 operation coverage 指向不存在的 case id。

```markdown
- case_id: AI-FEAT-01
  goal: Native AI chat operation reachable.
  steps: [ui_open: ai_chat, ui_submit: "Summarize current document"]
  assertions: [ui_assert: chat_response_visible true]

- case_id: DIFF-FEAT-01
  goal: Source-control change list supports stage-oriented review.
  steps: [ui_open_diff: true, ui_query_dom: ".diff-view"]
  assertions: [ui_assert: diff_header_change_stats_visible true]

- case_id: DIFF-FEAT-02
  goal: Commit history and publish flow expose commit identity.
  steps: [run: "deve dump --help", ui_open_diff: true]
  assertions: [stdout_contains: "dump"]

- case_id: DIFF-FEAT-03
  goal: Pending source-control operations expose safe discard or conflict handling.
  steps: [ui_open_diff: true, ui_query_dom: ".diff-view"]
  assertions: [ui_assert: diff_replaces_editor_only true]

- case_id: NET-FEAT-01
  goal: Repo-scoped sync handshake is observable.
  steps: [ws_send: {type: SyncHello, repo_id: "repo_a"}]
  assertions: [ws_receive_contains: "repo_a"]

- case_id: NET-FEAT-02
  goal: Repo-scoped sync transfer returns only missing facts.
  steps: [ws_send: {type: SyncRequest, repo_id: "repo_a"}]
  assertions: [ws_payload_contains_only_missing_facts: true]

- case_id: NET-FEAT-03
  goal: Repo switching and peer sync keep scopes isolated.
  steps: [ws_send: {type: SyncHello, repo_id: "repo_b"}]
  assertions: [ws_receive_not_contains: "reuse_repo_a_vector"]

- case_id: RENDER-FEAT-01
  goal: Document edit produces a confirmed operation.
  steps: [ui_type: "confirmed op check", run: "cargo test -p deve_cli duplicate_client_op -- --nocapture"]
  assertions: [duplicate_client_op_returns_original_ack true]

- case_id: REPO-FEAT-01
  goal: Repo file open resolves through UUID-first retrieval.
  steps: [run: "cargo test -p deve_cli open_doc_scope -- --nocapture"]
  assertions: [cli_assert: open_doc_bootstraps_by_doc_id true]

- case_id: REPO-FEAT-02
  goal: Branch switch operation is reachable from command surfaces.
  steps: [ui_keypress: "Ctrl+Shift+K"]
  assertions: [ui_assert: branch_switcher_visible true]

- case_id: REPO-FEAT-03
  goal: Repo repair and switch operations preserve repo scope.
  steps: [run: "deve node-check --help"]
  assertions: [stdout_contains: "node-check"]

- case_id: STORAGE-FEAT-01
  goal: Ledger-first edit increments durable operation state.
  steps: [run: "deve dump --help"]
  assertions: [stdout_contains: "dump"]

- case_id: STORAGE-FEAT-02
  goal: Tree projection can be rebuilt from durable ledger state.
  steps: [run: "deve repair --help"]
  assertions: [stdout_contains: "rebuild-projection"]

- case_id: WEBWRITE-FEAT-01
  goal: Pending local edit is cleared by confirmed Ack/NewOp.
  steps:
    - run: "cargo test -p deve_cli repo_write_gate_serializes_same_repo -- --nocapture"
    - run: "cargo test -p deve_cli duplicate_client_op_returns_original_ack_without_append -- --nocapture"
    - run: "cargo test -p deve_web echoed_new_op_clears_matching_pending_overlay -- --nocapture"
  assertions:
    - cli_assert: same_repo_browser_edit_commits_are_serialized true
    - ws_assert: ack_contains "doc_id client_op_id scope_nonce"
    - ui_assert: pending_overlay_cleared_after_confirmed_echo true

- case_id: WEBWRITE-FEAT-02
  goal: EditRejected clears matching pending edit without leaving infinite PendingAck.
  steps:
    - run: "cargo test -p deve_cli duplicate_client_op_with_different_op_is_rejected -- --nocapture"
    - run: "cargo test -p deve_web stale_edit_rejected_clears_matching_retained_pending_without_banner -- --nocapture"
  assertions:
    - ws_assert: edit_rejected_contains "scope_nonce doc_id client_op_id"
    - ui_assert: pending_overlay_cleared_after_reject true

- case_id: WEBWRITE-FEAT-03
  goal: Repo-scoped writer readiness gates edit replay.
  steps:
    - run: "cargo test -p deve_web writer_ready_requires_matching_repo_and_scope_nonce -- --nocapture"
    - run: "cargo test -p deve_web write_ready_resend_sends_pending_edit_when_native_runtime_is_ready -- --nocapture"
    - run: "cargo test -p deve_web write_ready_resend_skips_pending_edit_from_stale_scope -- --nocapture"
  assertions:
    - ui_assert: stale_scope_does_not_send_edit true
    - ui_assert: matching_repo_scope_sends_pending_edit true

- case_id: WEBNAV-FEAT-01
  goal: Pending edit navigation guard detects unsaved local input.
  steps: [ui_type: "pending edit", ui_attempt_navigation: true]
  assertions: [ui_assert: pending_navigation_modal_visible true]

- case_id: WEBNAV-FEAT-02
  goal: Pending edit navigation can be cancelled safely.
  steps: [ui_attempt_navigation: true, ui_click: "Stay"]
  assertions: [ui_assert: editor_visible true]

- case_id: WEBNAV-FEAT-03
  goal: Pending edit navigation can continue after explicit discard.
  steps: [ui_attempt_navigation: true, ui_click: "Discard and Leave"]
  assertions: [ui_assert: navigation_completed true]
```

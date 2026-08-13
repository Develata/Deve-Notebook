// In-crate `server` test modules, grouped on disk under tests/<family>/.
// Declarations stay flat children of `server` via #[path], so every test
// body keeps reaching the code under test through `super::` unchanged;
// only the file layout is organized by feature family.
#[path = "tests/catalog_repo_support.rs"]
mod catalog_repo_support;
#[path = "tests/ai_settings_http_test.rs"]
mod ai_settings_http_test;
#[path = "tests/docs/docs_copy_contract_test.rs"]
mod docs_copy_contract_test;
#[path = "tests/docs/docs_create_bootstrap_test.rs"]
mod docs_create_bootstrap_test;
#[path = "tests/docs/docs_create_test.rs"]
mod docs_create_test;
#[path = "tests/docs/docs_dir_copy_test.rs"]
mod docs_dir_copy_test;
#[path = "tests/docs/docs_projection_repair_test.rs"]
mod docs_projection_repair_test;
#[path = "tests/docs/docs_seed_test_support.rs"]
mod docs_seed_test_support;
#[path = "tests/docs/docs_test_support.rs"]
mod docs_test_support;
#[path = "tests/document/document_bootstrap_test_support.rs"]
mod document_bootstrap_test_support;
#[path = "tests/document/document_local_scope_test_support.rs"]
mod document_local_scope_test_support;
#[path = "tests/document/document_remote_scope_state_test_support.rs"]
mod document_remote_scope_state_test_support;
#[path = "tests/document/document_remote_scope_test.rs"]
mod document_remote_scope_test;
#[path = "tests/document/document_remote_scope_test_support.rs"]
mod document_remote_scope_test_support;
#[path = "tests/document/document_scope_bootstrap_test.rs"]
mod document_scope_bootstrap_test;
#[path = "tests/edit/edit_idempotency_test.rs"]
mod edit_idempotency_test;
#[path = "tests/edit/edit_message_test_support.rs"]
mod edit_message_test_support;
#[path = "tests/edit/edit_projection_ack_test.rs"]
mod edit_projection_ack_test;
#[path = "tests/edit/edit_runtime_repair_test.rs"]
mod edit_runtime_repair_test;
#[path = "tests/edit/edit_scope_binding_test.rs"]
mod edit_scope_binding_test;
#[path = "tests/edit/edit_scope_test.rs"]
mod edit_scope_test;
#[path = "tests/edit/edit_state_test_support.rs"]
mod edit_state_test_support;
#[path = "tests/key_exchange/key_exchange_message_test_support.rs"]
mod key_exchange_message_test_support;
#[path = "tests/key_exchange/key_exchange_scope_test.rs"]
mod key_exchange_scope_test;
#[path = "tests/key_exchange/key_exchange_test.rs"]
mod key_exchange_test;
#[path = "tests/key_exchange/key_exchange_test_support.rs"]
mod key_exchange_test_support;
#[path = "tests/listing/list_docs_scope_test/mod.rs"]
mod list_docs_scope_test;
#[path = "tests/listing/listing_scope_binding_test/mod.rs"]
mod listing_scope_binding_test;
#[path = "tests/listing/listing_scope_cleanup_test/mod.rs"]
mod listing_scope_cleanup_test;
#[path = "tests/listing/listing_scope_remote_branch_test.rs"]
mod listing_scope_remote_branch_test;
#[path = "tests/listing/listing_shadow_scope_catalog_test.rs"]
mod listing_shadow_scope_catalog_test;
#[path = "tests/listing/listing_shadow_scope_test/mod.rs"]
mod listing_shadow_scope_test;
#[path = "tests/listing/listing_shadow_scope_test_extra.rs"]
mod listing_shadow_scope_test_extra;
#[path = "tests/open_doc/open_doc_invalid_delta_test.rs"]
mod open_doc_invalid_delta_test;
#[path = "tests/open_doc/open_doc_invalid_delta_test_support.rs"]
mod open_doc_invalid_delta_test_support;
#[path = "tests/open_doc/open_doc_scope_test.rs"]
mod open_doc_scope_test;
#[path = "tests/open_doc/open_doc_snapshot_test.rs"]
mod open_doc_snapshot_test;
#[path = "tests/repo_scope/repo_scope_recovery_support.rs"]
mod repo_scope_recovery_support;
#[path = "tests/repo_scope/repo_scope_recovery_test.rs"]
mod repo_scope_recovery_test;
#[path = "tests/repo_scope/repo_scope_recovery_test_extra/mod.rs"]
mod repo_scope_recovery_test_extra;
#[path = "tests/repo_scope/repo_scope_remote_selector_test.rs"]
mod repo_scope_remote_selector_test;
#[path = "tests/repo_scope/repo_scope_runtime_selector_test.rs"]
mod repo_scope_runtime_selector_test;
#[path = "tests/repo_scope/repo_scope_test/mod.rs"]
mod repo_scope_test;
#[path = "tests/source_control/source_control_changes_identity_test/mod.rs"]
mod source_control_changes_identity_test;
#[path = "tests/source_control/source_control_commit_diff_test.rs"]
mod source_control_commit_diff_test;
#[path = "tests/source_control/source_control_git_import_conflict_test/mod.rs"]
mod source_control_git_import_conflict_test;
#[path = "tests/source_control/source_control_git_import_roundtrip_test.rs"]
mod source_control_git_import_roundtrip_test;
#[path = "tests/source_control/source_control_git_import_test_support.rs"]
mod source_control_git_import_test_support;
#[path = "tests/source_control/source_control_http_test/mod.rs"]
mod source_control_http_test;
#[path = "tests/source_control/source_control_local_commit_scope_test/mod.rs"]
mod source_control_local_commit_scope_test;
#[path = "tests/source_control/source_control_local_scope_test/mod.rs"]
mod source_control_local_scope_test;
#[path = "tests/source_control/source_control_remote_scope_test/mod.rs"]
mod source_control_remote_scope_test;
#[path = "tests/source_control/source_control_remote_selector_test.rs"]
mod source_control_remote_selector_test;
#[path = "tests/source_control/source_control_scope_binding_test/mod.rs"]
mod source_control_scope_binding_test;
#[path = "tests/source_control/source_control_scope_test/mod.rs"]
mod source_control_scope_test;
#[path = "tests/source_control/source_control_scope_test_support.rs"]
mod source_control_scope_test_support;
#[path = "tests/source_control/source_control_test_support.rs"]
mod source_control_test_support;
#[path = "tests/switcher/switcher_branch_scope_test/mod.rs"]
mod switcher_branch_scope_test;
#[path = "tests/switcher/switcher_branch_scope_test_extra.rs"]
mod switcher_branch_scope_test_extra;
#[path = "tests/switcher/switcher_branch_scope_test_fail_closed.rs"]
mod switcher_branch_scope_test_fail_closed;
#[path = "tests/switcher/switcher_branch_test/mod.rs"]
mod switcher_branch_test;
#[path = "tests/switcher/switcher_current_scope_binding_test.rs"]
mod switcher_current_scope_binding_test;
#[path = "tests/switcher/switcher_current_scope_remote_missing_test.rs"]
mod switcher_current_scope_remote_missing_test;
#[path = "tests/switcher/switcher_current_scope_remote_test.rs"]
mod switcher_current_scope_remote_test;
#[path = "tests/switcher/switcher_current_scope_test/mod.rs"]
mod switcher_current_scope_test;
#[path = "tests/switcher/switcher_exact_selector_fail_closed_test.rs"]
mod switcher_exact_selector_fail_closed_test;
#[path = "tests/switcher/switcher_exact_selector_test/mod.rs"]
mod switcher_exact_selector_test;
#[path = "tests/switcher/switcher_scope_rebind_test.rs"]
mod switcher_scope_rebind_test;
#[path = "tests/switcher/switcher_test_support.rs"]
mod switcher_test_support;
#[path = "tests/sync/sync_delete_peer_test_support.rs"]
mod sync_delete_peer_test_support;
#[path = "tests/sync/sync_hello_browser_scope_test.rs"]
mod sync_hello_browser_scope_test;
#[path = "tests/sync/sync_hello_browser_test.rs"]
mod sync_hello_browser_test;
#[path = "tests/sync/sync_hello_rebind_test.rs"]
mod sync_hello_rebind_test;
#[path = "tests/sync/sync_hello_scope_test.rs"]
mod sync_hello_scope_test;
#[path = "tests/sync/sync_hello_test.rs"]
mod sync_hello_test;
#[path = "tests/sync/sync_hello_test_support.rs"]
mod sync_hello_test_support;
#[path = "tests/sync/sync_scope_cleanup_browser_test.rs"]
mod sync_scope_cleanup_browser_test;
#[path = "tests/sync/sync_scope_cleanup_test.rs"]
mod sync_scope_cleanup_test;
#[path = "tests/sync/sync_scope_cleanup_test_support.rs"]
mod sync_scope_cleanup_test_support;
#[path = "tests/sync/sync_transfer_push_test.rs"]
mod sync_transfer_push_test;
#[path = "tests/sync/sync_transfer_scope_test.rs"]
mod sync_transfer_scope_test;
#[path = "tests/sync/sync_transfer_scope_test_support.rs"]
mod sync_transfer_scope_test_support;
#[path = "tests/sync/sync_transfer_snapshot_test.rs"]
mod sync_transfer_snapshot_test;
#[path = "tests/ws_acceptance/ws_edit_flow_acceptance_support.rs"]
mod ws_edit_flow_acceptance_support;
#[path = "tests/ws_acceptance/ws_edit_readback_acceptance_test.rs"]
mod ws_edit_readback_acceptance_test;
#[path = "tests/ws_acceptance/ws_edit_success_acceptance_test.rs"]
mod ws_edit_success_acceptance_test;
#[path = "tests/ws_acceptance/ws_edit_writer_gate_acceptance_test.rs"]
mod ws_edit_writer_gate_acceptance_test;
#[path = "tests/ws_acceptance/ws_key_exchange_acceptance_test.rs"]
mod ws_key_exchange_acceptance_test;
#[path = "tests/ws_acceptance/ws_protocol_acceptance_support.rs"]
mod ws_protocol_acceptance_support;
#[path = "tests/ws_acceptance/ws_protocol_acceptance_test.rs"]
mod ws_protocol_acceptance_test;
#[path = "tests/ws_acceptance/ws_register_writer_acceptance_test.rs"]
mod ws_register_writer_acceptance_test;
#[path = "tests/ws_acceptance/ws_source_control_acceptance_support.rs"]
mod ws_source_control_acceptance_support;
#[path = "tests/ws_acceptance/ws_source_control_acceptance_test.rs"]
mod ws_source_control_acceptance_test;
#[path = "tests/ws_acceptance/ws_sync_hello_acceptance_test.rs"]
mod ws_sync_hello_acceptance_test;
#[path = "tests/ws_acceptance/ws_sync_hello_reject_acceptance_test.rs"]
mod ws_sync_hello_reject_acceptance_test;
#[path = "tests/ws_acceptance/ws_sync_transfer_reject_acceptance_test.rs"]
mod ws_sync_transfer_reject_acceptance_test;

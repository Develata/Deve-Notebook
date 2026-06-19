// apps\cli\src\bin
//! plan_ref: infra

use anyhow::{Context, Result, bail};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let command = env::args().nth(1).unwrap_or_else(|| "--help".to_string());
    let root = repo_root()?;

    match command.as_str() {
        "storage-repo" => run_storage_repo(&root),
        "network" => run_network(&root),
        "all" => {
            run_storage_repo(&root)?;
            run_network(&root)
        }
        "-h" | "--help" | "help" => {
            println!("Usage: deve_baseline <storage-repo|network|all>");
            Ok(())
        }
        other => {
            bail!("unknown baseline check '{other}'. expected one of: storage-repo, network, all")
        }
    }
}

fn repo_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("failed to resolve repository root from CARGO_MANIFEST_DIR")
}

struct CheckContext<'a> {
    root: &'a Path,
    label: &'static str,
}

impl<'a> CheckContext<'a> {
    fn new(root: &'a Path, label: &'static str) -> Self {
        Self { root, label }
    }

    fn contains(&self, rel: &str, text: &str) -> Result<()> {
        let content = self.read(rel)?;
        if content.contains(text) {
            Ok(())
        } else {
            bail!("{}: missing '{}' in {}", self.label, text, display_rel(rel))
        }
    }

    fn absent(&self, rel: &str, text: &str) -> Result<()> {
        let content = self.read(rel)?;
        if content.contains(text) {
            bail!(
                "{}: unexpected '{}' in {}",
                self.label,
                text,
                display_rel(rel)
            )
        } else {
            Ok(())
        }
    }

    fn case_contains(&self, acceptance: &str, case_id: &str, text: &str) -> Result<()> {
        let content = self.read(acceptance)?;
        let block = case_block(&content, case_id)
            .with_context(|| format!("{}: missing case block {case_id}", self.label))?;
        if block.contains(text) {
            Ok(())
        } else {
            bail!("{}: missing '{}' in {}", self.label, text, case_id)
        }
    }

    fn read(&self, rel: &str) -> Result<String> {
        fs::read_to_string(self.root.join(rel))
            .with_context(|| format!("{}: failed to read {}", self.label, display_rel(rel)))
    }

    fn ok(&self) {
        println!("{}: ok", self.label);
    }
}

fn display_rel(rel: &str) -> String {
    rel.replace('\\', "/")
}

fn case_block(content: &str, case_id: &str) -> Result<String> {
    let needle = format!("case_id: {case_id}");
    let mut in_case = false;
    let mut block = String::new();

    for line in content.lines() {
        if line.contains(&needle) {
            in_case = true;
        }
        if in_case && line.starts_with("- case_id: ") && !line.contains(&needle) {
            break;
        }
        if in_case {
            block.push_str(line);
            block.push('\n');
        }
    }

    if block.is_empty() {
        bail!("case block not found")
    }
    Ok(block)
}

fn run_storage_repo(root: &Path) -> Result<()> {
    let ctx = CheckContext::new(root, "storage-repo-baseline-check");
    let acceptance = "docs/acceptance-cases/07_storage_repo.md";

    for_pairs(STORAGE_CASE_CHECKS, |case_id, text| {
        ctx.case_contains(acceptance, case_id, text)
    })?;
    for_line(STORAGE_CASES_WITH_BASELINE_SCRIPT, |case_id| {
        ctx.case_contains(
            acceptance,
            case_id,
            "run: scripts/check-storage-repo-baseline.sh",
        )
    })?;
    for_pairs(STORAGE_CONTAINS, |rel, text| ctx.contains(rel, text))?;
    for_pairs(STORAGE_ABSENT, |rel, text| ctx.absent(rel, text))?;

    ctx.ok();
    Ok(())
}

fn run_network(root: &Path) -> Result<()> {
    let ctx = CheckContext::new(root, "network-baseline-check");

    for_pairs(NETWORK_CONTAINS, |rel, text| ctx.contains(rel, text))?;
    for_pairs(NETWORK_ABSENT, |rel, text| ctx.absent(rel, text))?;

    ctx.ok();
    Ok(())
}

fn for_pairs(spec: &str, mut check: impl FnMut(&str, &str) -> Result<()>) -> Result<()> {
    for (line_no, line) in spec.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let (left, right) = line
            .split_once('\t')
            .with_context(|| format!("invalid baseline pair spec at line {}", line_no + 1))?;
        check(left, right)?;
    }
    Ok(())
}

fn for_line(spec: &str, mut check: impl FnMut(&str) -> Result<()>) -> Result<()> {
    for line in spec.lines().filter(|line| !line.is_empty()) {
        check(line)?;
    }
    Ok(())
}

const STORAGE_CASE_CHECKS: &str = r###"STORE-001	cargo test -p deve_cli init_creates_trinity_workspace_layout -- --nocapture
STORE-001	cargo test -p deve_cli projection_locator_init_writes_locator_without_vault_path_config -- --nocapture
STORE-001	cargo test -p deve_core trinity_dir_structure_after_init -- --nocapture
STORE-001	cargo test -p deve_core projection_locator_toml_roundtrip -- --nocapture
STORE-002	cargo test -p deve_core init_allocates_collision_safe_repo_name_for_same_name_different_url -- --nocapture
STORE-003	cargo test -p deve_core required_redb_tables_exist_after_init -- --nocapture
STORE-004	cargo test -p deve_core snapshot_respects_depth_limit -- --nocapture
STORE-005	cargo test -p deve_core edit_round_trip_reconstructs_content -- --nocapture
STORE-005	cargo test -p deve_core global_seq_increases -- --nocapture
STORE-006	cargo test -p deve_cli markdown_export_preserves_user_frontmatter_without_system_metadata -- --nocapture
STORE-007	cargo test -p deve_core watcher_records_create_modify_delete_candidates -- --nocapture
STORE-007	cargo test -p deve_core watcher_duplicate_start_fails_and_can_restart_after_stop -- --nocapture
STORE-007	cargo test -p deve_core internal_repo_path_uses_segment_semantics -- --nocapture
STORE-007	cargo test -p deve_core --test watcher_internal_ignore -- --nocapture
STORE-007	cargo test -p deve_core --test watcher_internal_ignore watcher_respects_deveignore_for_matching_markdown -- --nocapture
STORE-007	cargo test -p deve_core --test watcher_internal_ignore watcher_startup_scan_respects_deveignore -- --nocapture
STORE-008	cargo test -p deve_cli recover_rebuilds_workspace_files_from_ledger -- --nocapture
STORE-008	cargo test -p deve_core rebuild_projection_recovers_when_node_projection_is_missing -- --nocapture
STORE-009	cargo test -p deve_cli document_scope_bootstrap -- --nocapture
STORE-009	cargo test -p deve_cli open_doc_scope -- --nocapture
STORE-009	cargo test -p deve_cli resolve_target_prefers_doc_id_over_stale_path -- --nocapture
STORE-010	cargo test -p deve_core --test path_normalize_structure_test -- --nocapture
STORE-012	run: scripts/check-repo-file-ops-baseline.sh
STORE-013	cargo test -p deve_cli degraded_local -- --nocapture
STORE-013	cargo test -p deve_cli browser_writer_registration_rejects_broken_workspace_identity -- --nocapture
STORE-013	cargo test -p deve_core source_control_write_gate -- --nocapture
STORE-013	run: scripts/check-repo-file-ops-baseline.sh
STORE-014	cargo test -p deve_cli jsonl_roundtrip_is_monotonic_and_line_stable -- --nocapture
STORE-014	cargo test -p deve_cli includes_dir_structure_fact_in_export -- --nocapture
STORE-015	cargo test -p deve_cli edit_acknowledges_ledger_commit_when_workspace_writeback_fails -- --nocapture
STORE-015	cargo test -p deve_core --test durable_projection_fault_test -- --nocapture
STORE-016	cargo test -p deve_core notify_backend_error_requests_rescan -- --nocapture
STORE-016	cargo test -p deve_core notify_rescan_flag_requests_rescan -- --nocapture
STORE-016	cargo test -p deve_core watcher_rejects_zero_debounce_window -- --nocapture
STORE-016	cargo test -p deve_core dispatch_batch_collapses_modified_burst_by_content_hash -- --nocapture
STORE-017	cargo test -p deve_core remote_repo_catalog_calls_fail_closed_when_remotes_dir_is_missing -- --nocapture
STORE-017	cargo test -p deve_core remote_repo_listing_fails_closed_on_unexpected_non_redb_entry -- --nocapture
STORE-017	cargo test -p deve_cli quarantines_nil_shadow_repo_into_invalid_peer_dir -- --nocapture"###;

const STORAGE_CASES_WITH_BASELINE_SCRIPT: &str = r###"STORE-001
STORE-002
STORE-003
STORE-004
STORE-005
STORE-006
STORE-007
STORE-008
STORE-009
STORE-010
STORE-014
STORE-015
STORE-016
STORE-017"###;

const STORAGE_CONTAINS: &str = r###"apps/cli/src/commands/init.rs	fn init_creates_trinity_workspace_layout()
apps/cli/src/commands/repo_projection.rs	fn projection_locator_set_list_check_roundtrip()
apps/cli/src/commands/recover.rs	fn recover_rebuilds_workspace_files_from_ledger()
apps/cli/src/commands/export/tests.rs	fn markdown_export_preserves_user_frontmatter_without_system_metadata()
crates/core/tests/local_repo_metadata_repair_test.rs	fn init_allocates_collision_safe_repo_name_for_same_name_different_url()
crates/core/tests/store_acceptance_test.rs	SNAPSHOT_DATA
crates/core/tests/watcher_lifecycle.rs	fn watcher_duplicate_start_fails_and_can_restart_after_stop()
crates/core/tests/watcher_lifecycle.rs	fn watcher_rejects_zero_debounce_window()
crates/core/src/sync/watcher/backend/notify_impl.rs	fn notify_backend_error_requests_rescan()
crates/core/src/sync/watcher/backend/notify_impl.rs	fn notify_rescan_flag_requests_rescan()
crates/core/src/sync/watcher/dispatch_burst_test.rs	fn dispatch_batch_collapses_modified_burst_by_content_hash()
crates/core/src/utils/notegit.rs	fn internal_repo_path_uses_segment_semantics()
crates/core/src/utils/notegit.rs	.notegit-backup/state.json
crates/core/src/utils/notegit.rs	.git-backup/config
crates/core/tests/watcher_internal_ignore.rs	fn watcher_ignores_internal_notegit_paths()
crates/core/tests/watcher_internal_ignore.rs	fn watcher_ignores_internal_git_paths()
crates/core/tests/watcher_internal_ignore.rs	fn watcher_allows_notegit_backup_sibling_path()
crates/core/src/sync/watcher/filter.rs	!is_internal_repo_path(normalized)
crates/core/src/sync/watcher/mod.rs	registry::is_running(info.uuid)
crates/core/src/sync/watcher/mod.rs	registry::begin_stop(repo_id)
crates/core/src/sync/watcher/mod.rs	registry::finish_stop(repo_id)
crates/core/src/sync/watcher/mod.rs	stop_handle(rejected)?
crates/core/src/sync/watcher/registry.rs	WatcherSlot::Stopping
crates/core/src/ledger/manager/remote_repo_select.rs	let Some(info) = entry.info.as_ref() else
apps/cli/src/export_entries.rs	fn jsonl_roundtrip_is_monotonic_and_line_stable()
apps/cli/src/export_entries.rs	fn includes_dir_structure_fact_in_export()
apps/cli/src/server/tests/edit/edit_projection_ack_test.rs	fn edit_acknowledges_ledger_commit_when_workspace_writeback_fails()
apps/cli/src/server/handlers/document/edit_apply.rs	CommitOutcome::WritebackFailed
apps/cli/src/server/handlers/document/edit_apply.rs	emit_commit_outcome(
apps/cli/src/server/handlers/document/write_confirmation.rs	broadcast_and_ack_committed_edit(
apps/cli/src/server/handlers/document/write_confirmation.rs	report_projection_writeback_fault(
crates/core/src/sync/projection_fault_journal.rs	struct DurableProjectionFault
crates/core/tests/durable_projection_fault_test.rs	fn durable_projection_fault_survives_sync_manager_restart()
crates/core/tests/remote_repo_catalog_missing_test.rs	fn remote_repo_catalog_calls_fail_closed_when_remotes_dir_is_missing()
crates/core/tests/repo_catalog_entry_fail_closed_test.rs	fn remote_repo_listing_fails_closed_on_unexpected_non_redb_entry()
apps/cli/src/commands/repair/shadow.rs	fn quarantines_nil_shadow_repo_into_invalid_peer_dir()
scripts/check-repo-file-ops-baseline.sh	run_filter deve_web file_ops
scripts/check-repo-file-ops-baseline.sh	run_filter deve_web file_provider
scripts/check-repo-file-ops-baseline.sh	run_filter deve_cli docs_scope_nonce_gate
scripts/check-repo-file-ops-baseline.sh	run_filter deve_cli docs_create_test
scripts/check-repo-file-ops-baseline.sh	run_filter deve_cli docs_copy_contract
scripts/check-repo-file-ops-baseline.sh	run_filter deve_cli docs_dir_copy
scripts/check-repo-file-ops-baseline.sh	run_filter deve_cli docs_projection_repair
scripts/check-repo-file-ops-baseline.sh	run_filter deve_cli degraded_local
scripts/check-repo-file-ops-baseline.sh	run_filter deve_cli browser_writer_registration_rejects_broken_workspace_identity
scripts/check-repo-file-ops-baseline.sh	run_filter deve_core source_control_write_gate"###;

const STORAGE_ABSENT: &str = r###"crates/core/src/ledger/manager/remote_repo_select.rs	expect("validated readable")
docs/acceptance-cases/07_storage_repo.md	deve repo create
docs/acceptance-cases/07_storage_repo.md	deve db inspect
docs/acceptance-cases/07_storage_repo.md	deve doc edit
docs/acceptance-cases/07_storage_repo.md	deve dump --doc
docs/acceptance-cases/07_storage_repo.md	deve api call
docs/acceptance-cases/07_storage_repo.md	cargo test -p deve_core path_normalize_structure -- --nocapture
docs/acceptance-cases/07_storage_repo.md	deve path normalize
docs/acceptance-cases/07_storage_repo.md	deve recover --from-ledger
docs/acceptance-cases/07_storage_repo.md	powershell -Command
docs/acceptance-cases/07_storage_repo.md	dir "${DEVE_DATA_DIR}"
docs/acceptance-cases/07_storage_repo.md	type ${DEVE_DATA_DIR}"###;

const NETWORK_CONTAINS: &str = r###"apps/web/src/api/connection.rs	try_set_connection_status(&signals, ConnectionStatus::Connecting)
apps/web/src/api/connection.rs	try_set_connection_status(&signals, ConnectionStatus::Disconnected)
apps/web/src/api/connection/session.rs	try_set_session_status(&signals, ConnectionStatus::Disconnected)
apps/web/src/api/write_gate.rs	fn status_revokes_writer_ready(status: ConnectionStatus) -> bool
apps/web/src/api/write_gate.rs	!matches!(status, ConnectionStatus::Connected)
apps/web/src/api/service/tests.rs	writer_ready_is_cleared_on_disconnected_status
apps/web/src/api/service/tests.rs	writer_ready_is_cleared_on_unauthorized_status
apps/web/src/components/disconnect_overlay.rs	ConnectionStatus::Unauthorized | ConnectionStatus::Connected => None
apps/web/src/hooks/use_core/write_gate/logic.rs	ConnectionStatus::Disconnected => Some(RepoWriteBlock::Offline)
apps/web/src/hooks/use_core/write_gate/logic.rs	ConnectionStatus::Connecting => Some(RepoWriteBlock::Reconnecting)
apps/web/src/api/connection_urls.rs	format!("{}://{}/ws", ws_scheme, host)
apps/web/src/api/connection_urls.rs	cfg!(debug_assertions)
apps/web/src/api/connection_urls.rs	include_debug_fallbacks
apps/web/src/api/connection_urls.rs	if include_debug_fallbacks
apps/web/src/api/connection_urls.rs	ws_port
apps/web/src/api/connection_urls.rs	fn parse_ws_port(value: &str) -> Option<u16>
apps/web/src/api/connection_urls.rs	query_ws_port_rejects_invalid_or_zero_ports
apps/cli/src/server/router.rs	.route("/api/node/role", get(node_role_http::role))
apps/cli/src/server/router.rs	public = Router::new()
apps/cli/src/server/node_role_http.rs	"role": r.role
apps/cli/src/server/node_role_http.rs	"ws_port": r.ws_port
apps/cli/src/server/node_role_http.rs	"main_port": r.main_port
crates/core/src/protocol/frame.rs	pub const WS_PROTOCOL_VERSION: u16 = 9;
crates/core/src/protocol/frame.rs	pub const MIN_SUPPORTED_WS_PROTOCOL_VERSION: u16 = 9;
crates/core/src/protocol/frame.rs	pub const WS_FRAME_MAGIC: &[u8] = b"DEVEWSF3";
crates/core/src/protocol/frame.rs	missing WS frame magic
apps/cli/src/server/ws/receive/mod.rs	MISSING_WS_FRAME_MAGIC
apps/cli/src/server/ws/receive/tests/frame_errors.rs	Some("missing WS frame magic")
apps/cli/src/server/ws/receive/mod.rs	JSON WS text frames are disabled outside development debug mode
apps/cli/src/server/ws/receive/mod.rs	allow_ws_json_text_debug
apps/cli/src/server/ws/receive/mod.rs	ServerErrorCode::SyncVersionMismatch
apps/cli/src/server/ws/receive/mod.rs	ServerErrorCode::SyncInvalidPayload
apps/cli/src/server/ws/receive/mod.rs	DEVE_ALLOW_WS_JSON_TEXT
apps/cli/src/server/ws/receive/mod.rs	DEVE_ALLOW_LEGACY_WS_JSON
apps/cli/src/server/ws/receive/mod.rs	DEVE_ENV
apps/cli/src/server/ws/receive/tests/frame_errors.rs	unsupported_versioned_json_reports_version_mismatch
apps/cli/src/server/ws/receive/tests/frame_errors.rs	malformed_versioned_binary_reports_invalid_payload
apps/web/src/api/incoming/tests.rs	binary_malformed_versioned_payload_surfaces_protocol_error
apps/web/src/api/service.rs	writer_ready_scope_nonce
apps/web/src/api/service.rs	writer_ready_for(&self, repo_id: Option<&str>, scope_nonce: Option<u64>)
apps/web/src/hooks/use_core/effects/message_dispatch_write.rs	ws.mark_writer_ready(repo_id, scope_nonce, peer_id.as_str())
apps/web/src/hooks/use_core/status_summary.rs	PeerNotRegistered
apps/web/src/components/bottom_bar/status.rs	data-deve-peer-registration-retry="true"
apps/web/src/components/mobile_layout/footer_status.rs	data-deve-peer-registration-retry="mobile"
apps/web/src/hooks/use_core/effects/handshake/mod.rs	handshake_retry_nonce
apps/web/src/hooks/use_core/state_build/assemble.rs	build_retry_peer_registration_callback
apps/cli/src/server/session/writer.rs	pub scope_nonce: u64
apps/cli/src/server/session/scope.rs	writer_peer_id_for(&self, repo_id: &RepoId, scope_nonce: Option<u64>)
apps/cli/src/server/handlers/sync/writer/mod.rs	session.set_writer_identity(repo_id, peer_id.clone(), scope_nonce)
apps/cli/src/server/handlers/document/edit_checks.rs	.writer_peer_id_for(repo_id, requested_scope_nonce)
apps/cli/src/server/handlers/document/edit_apply.rs	with_repo_write_gate(repo_id, || append_client_edit_locked(input))
apps/cli/src/server/handlers/document/write_gate.rs	HashMap<RepoId, Arc<Mutex<()>>
apps/cli/src/server/handlers/document/write_gate.rs	fn repo_write_gate_serializes_same_repo()
docs/acceptance-cases/14_operation_flow_refs.md	cargo test -p deve_cli repo_write_gate_serializes_same_repo -- --nocapture
apps/cli/src/server/ws/route/core_scoped.rs	document::handle_edit(
apps/cli/src/server/handlers/sync/snapshot.rs	snapshot_kind: Some("full".to_string())
crates/core/src/protocol/session_proof.rs	pub struct SessionProof
crates/core/src/protocol/client.rs	peer_pubkey: Vec<u8>
crates/core/src/protocol/client.rs	session_proof: SessionProof
apps/cli/src/server/ws/route/mod.rs	session_proof
scripts/smoke-runtime-happy-path.sh	run_test deve_cli ws_endpoint_sync_hello_uses_switched_repo_scope
scripts/smoke-runtime-happy-path.sh	run_test deve_cli ws_endpoint_register_writer_after_sync_hello_returns_write_ready
apps/cli/src/server/tests/ws_acceptance/ws_sync_hello_acceptance_test.rs	async fn ws_endpoint_sync_hello_uses_switched_repo_scope
apps/cli/src/server/tests/ws_acceptance/ws_register_writer_acceptance_test.rs	async fn ws_endpoint_register_writer_after_sync_hello_returns_write_ready
scripts/smoke-runtime-happy-path.sh	run_test deve_cli ws_open_doc_and_history_read_back_registered_edit
apps/cli/src/server/tests/ws_acceptance/ws_edit_readback_acceptance_test.rs	async fn ws_open_doc_and_history_read_back_registered_edit
apps/cli/src/server/test_modules.rs	mod open_doc_scope_test;
scripts/check-storage-repo-baseline.sh	case_contains STORE-009 "cargo test -p deve_cli open_doc_scope -- --nocapture"
apps/cli/src/server/tests/sync/sync_transfer_scope_test.rs	async fn non_browser_sync_request_uses_bound_sync_scope_nonce_for_push
apps/cli/src/server/tests/sync/sync_transfer_scope_test.rs	async fn sync_request_preserves_requested_source_peer_in_push
apps/cli/src/server/tests/ws_acceptance/ws_sync_transfer_reject_acceptance_test.rs	async fn ws_sync_request_requires_sync_hello_scope
apps/cli/src/server/tests/ws_acceptance/ws_sync_transfer_reject_acceptance_test.rs	async fn ws_sync_request_rejects_wrong_repo_after_sync_hello
apps/cli/src/server/tests/sync/sync_transfer_scope_test.rs	async fn non_browser_snapshot_request_uses_bound_sync_scope_nonce_for_push
apps/cli/src/server/tests/sync/sync_transfer_snapshot_test.rs	async fn snapshot_request_exports_requested_shadow_source
apps/cli/src/server/tests/sync/sync_transfer_snapshot_test.rs	async fn snapshot_request_rejects_unoffered_source
apps/cli/src/server/handlers/sync/snapshot.rs	snapshot_kind: Some("full".to_string())
apps/cli/src/server/tests/sync/sync_hello_browser_test.rs	async fn browser_sync_hello_rejects_stale_scope_nonce
apps/cli/src/server/tests/sync/sync_hello_browser_scope_test.rs	async fn browser_sync_hello_rejects_stale_active_db_binding
apps/cli/src/server/tests/sync/sync_hello_browser_scope_test.rs	async fn browser_sync_hello_rejects_stale_bound_repo_and_writer_identity
apps/cli/src/server/tests/sync/sync_transfer_push_test.rs	async fn manual_sync_push_buffers_without_applying_remote_ops
apps/cli/src/server/tests/sync/sync_transfer_push_test.rs	async fn sync_push_uses_message_source_peer_for_shadow_write
apps/cli/src/server/tests/sync/sync_transfer_push_test.rs	async fn sync_push_does_not_pollute_transport_or_local_ledger
apps/cli/src/server/tests/sync/sync_transfer_snapshot_test.rs	async fn sync_push_snapshot_uses_message_source_peer_for_shadow_replace
apps/cli/src/server/tests/ws_acceptance/ws_sync_transfer_reject_acceptance_test.rs	async fn ws_sync_push_rejects_unrequested_source
apps/cli/src/server/tests/sync/sync_transfer_push_test.rs	async fn sync_push_rejects_unrequested_relay_source
apps/cli/src/server/tests/sync/sync_transfer_push_test.rs	async fn sync_push_rejects_relay_forged_source_proof
apps/cli/src/server/tests/sync/sync_transfer_snapshot_test.rs	async fn sync_push_snapshot_rejects_relay_forged_source_proof
crates/core/src/protocol/sync_push_header/tests.rs	fn source_proof_signing_rejects_wrong_source_key
crates/core/src/protocol/sync_push_header/tests.rs	fn source_proof_rejects_payload_tamper
scripts/check-ws-structured-errors.sh	ws-structured-errors-check: ok
apps/cli/src/server/ws/route/core_scoped/tests.rs	core_scoped_scope_nonce_gate_rejects_missing_scope_before_handler
apps/cli/src/server/ws/route/core_scoped/tests.rs	core_scoped_scope_nonce_gate_rejects_stale_scope_before_handler
scripts/check-auth-unauthorized-state.sh	auth-unauthorized-check: ok
scripts/smoke-runtime-recovery-path.sh	deve_web \
scripts/smoke-runtime-recovery-path.sh	status_summary \
scripts/smoke-runtime-recovery-path.sh	auth_probe \
docs/plan/07_network.md	Full Peer Mesh v1
docs/plan/07_network.md	FullPeer
docs/plan/07_network.md	Admission
docs/plan/07_network.md	shadow repo
docs/features/05_network.md	Full Peer Mesh v1
docs/features/05_network.md	Shadow 与显式合并边界
docs/acceptance-cases/06_network.md	case_id: NET-014
docs/acceptance-cases/06_network.md	case_id: NET-015
docs/acceptance-cases/06_network.md	case_id: NET-016
docs/acceptance-bindings.tsv	NET-014|manual-network|docs/features/05_network.md
docs/acceptance-bindings.tsv	NET-015|manual-network|docs/features/05_network.md
docs/acceptance-bindings.tsv	NET-016|manual-network|docs/features/05_network.md
apps/cli/src/server/ws/auth.rs	pub(super) enum WsAdmission
apps/cli/src/server/ws/auth.rs	FullPeer
apps/cli/src/server/ws/auth.rs	DEVE_P2P_INBOUND_TOKEN
apps/cli/src/server/ws/auth.rs	fn bearer_token_matches(token: &str, expected: &str) -> bool
apps/cli/src/server/ws/auth.rs	if bearer_token_matches(token, &expected)
apps/cli/src/server/ws/mod.rs	admission.browser_auth_session().cloned()
apps/cli/src/server/ws/mod.rs	session.mark_browser_session()
apps/cli/src/server/ws/mod.rs	session.bind_auth_session(auth_session_id)
apps/cli/src/server/router.rs	.route("/ws", get(ws::ws_handler))
apps/cli/src/server/p2p.rs	spawn_mesh_connectors
apps/cli/src/server/p2p/connect.rs	P2P mesh connector handshake completed
apps/cli/src/server/p2p/connect.rs	encode_client_binary(&hello)
apps/cli/src/server/p2p/source_sets.rs	with_strict_engine(repo_id
docs/acceptance-cases/06_network.md	p2p_connector_static_token_header_errors_are_terminal
docs/acceptance-cases/06_network.md	p2p_connector_retry_backoff_starts_at_one_second
docs/acceptance-cases/06_network.md	p2p_connector_jitter_uses_peer_identity_not_label
docs/acceptance-cases/06_network.md	load_checked_fails_closed_on_invalid_p2p_peer_id_identity
docs/acceptance-cases/06_network.md	static_p2p_peer_id_human_label_rejected
crates/core/src/config/validation.rs	canonical identity peer id
docker-compose.mesh.yml	00000000000a
docker-compose.mesh.yml	00000000000b
docs/acceptance-cases/06_network.md	p2p_exchange_rejects_duplicate_sync_hello
docs/acceptance-cases/06_network.md	p2p_connector_duplicate_sync_hello_is_terminal
apps/cli/src/server/p2p/tests/exchange.rs	async fn p2p_exchange_rejects_duplicate_sync_hello
apps/cli/src/server/p2p_connector/tests.rs	fn p2p_connector_duplicate_sync_hello_is_terminal
apps/cli/src/server/p2p_connector/tests.rs	fn p2p_connector_retry_backoff_starts_at_one_second
apps/cli/src/server/p2p_connector/tests.rs	fn p2p_connector_jitter_uses_peer_identity_not_label
apps/cli/src/server/p2p_connector.rs	"token_invalid"
apps/cli/src/server/p2p_connector.rs	"duplicate_sync_hello"
apps/cli/src/server/p2p_connector/tests.rs	fn p2p_connector_static_token_header_errors_are_terminal"###;

const NETWORK_ABSENT: &str = r###"apps/web/src/api/connection_urls.rs	Scanning ports
apps/cli/src/server/ws/auth.rs	token == expected"###;

#[cfg(test)]
mod tests {
    use super::case_block;

    #[test]
    fn baseline_case_block_stops_at_next_case() {
        let content = "- case_id: STORE-001\n  steps:\n    - run: one\n- case_id: STORE-002\n  steps:\n    - run: two\n";
        let block = case_block(content, "STORE-001").expect("case block");

        assert!(block.contains("run: one"));
        assert!(!block.contains("run: two"));
    }

    #[test]
    fn baseline_case_block_reports_missing_case() {
        assert!(case_block("- case_id: STORE-001\n", "STORE-999").is_err());
    }
}
